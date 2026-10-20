using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Threading;
using UnityEngine;

// These adapters isolate the scratch player from the normal Unity profile.
public static class G11Prefs
{
    static readonly Dictionary<string,string> values = new Dictionary<string,string>();
    public static void SetInt(string key,int value) { values[key]=value.ToString(CultureInfo.InvariantCulture); }
    public static void SetFloat(string key,float value) { values[key]=value.ToString("R",CultureInfo.InvariantCulture); }
    public static void SetString(string key,string value) { values[key]=value; }
    public static int GetInt(string key) { return GetInt(key,0); }
    public static int GetInt(string key,int value) { return values.ContainsKey(key)?int.Parse(values[key],CultureInfo.InvariantCulture):value; }
    public static float GetFloat(string key) { return GetFloat(key,0); }
    public static float GetFloat(string key,float value) { return values.ContainsKey(key)?float.Parse(values[key],CultureInfo.InvariantCulture):value; }
    public static string GetString(string key) { return GetString(key,""); }
    public static string GetString(string key,string value) { return values.ContainsKey(key)?values[key]:value; }
    public static bool HasKey(string key) { return values.ContainsKey(key); }
    public static void DeleteKey(string key) { values.Remove(key); }
    public static void DeleteAll() { values.Clear(); }
    public static void Save() { }
}

public sealed class G11Capture : MonoBehaviour
{
    [Serializable] public class Setup
    {
        public int seed, removed_driven_rims;
        public bool auto_clutch, auto_shift;
        public string support, height, engine, fluids, surface;
        public Vector3 velocity_mps;
        public float load_multiplier;
    }
    [Serializable] public class Control
    {
        public int frame;
        public float throttle, brake, clutch, steer, motor_scale, service_brake_fraction, handbrake_fraction;
        public bool ignition_button_held, handbrake_press;
        public string gear_press;
    }
    [Serializable] public class Scenario
    {
        public string id, probe_kind;
        public int duration_frames;
        public Setup setup;
        public Initial setup_state;
        public bool has_setup_state;
        public Control[] controls;
        public G11AngularProbe.Input angular;
        public G11ContactProbe.Input contact;public G11AngularProbe.BodyInput contact_body_input;public bool contact_body_input_from_source;
        public float[] engine_torque_holdout_inputs;
    }
    [Serializable] public struct Wheel
    {
        public bool contact;
        public float rpm, motor_torque, brake_torque, steer_angle, radius, suspension_distance;
        public Vector3 pose, contact_point, contact_normal, forward, sideways;
        public Quaternion rotation;
        public float suspension_offset_m, force_n, forward_slip, sideways_slip;
        public string surface;
    }
    [Serializable] public struct Sample
    {
        public string phase;
        public int sequence, step, frame, input_sequence, car_sequence, engine_sequence, rpm_sequence;
        public float player_stored_dt,player_consumed_dt;
        public int player_update_sequence;
        public float time, realtime, delta_time, fixed_dt, rpm, average_rpm, clutch, throttle, brake, handbrake;
        public float engine_output_nm, engine_brake_nm;
        public int gear, installed_driven_rims, torque_divisor;
        public G11Controls.State controls;
        public Vector3 position, velocity, angular_velocity;
        public Quaternion rotation;
        public Wheel[] wheels;
        public G11Geometry.NativeWheel[] native_wheels;
        public G11AngularProbe.State angular_state;public G11ContactProbe.State contact_state;
    }
    [Serializable] public class WheelParameters
    {
        public string path;
        public Vector3 local_position, center;
        public Quaternion local_rotation;
        public float radius, mass, damping, suspension_distance, force_application_distance;
        public float spring_n_per_m, damper_n_s_per_m, spring_target;
        public Friction forward, sideways;
        public bool driven, steer, service_brake, handbrake;
    }
    [Serializable] public class Friction
    {
        public float extremum_slip, extremum_value, asymptote_slip, asymptote_value, stiffness;
        public Friction(WheelFrictionCurve source)
        {
            extremum_slip=source.extremumSlip; extremum_value=source.extremumValue;
            asymptote_slip=source.asymptoteSlip; asymptote_value=source.asymptoteValue; stiffness=source.stiffness;
        }
    }
    [Serializable] public class Key
    {
        public float time, value, in_tangent, out_tangent, in_weight, out_weight;
        public int weighted_mode;
        public Key(Keyframe source)
        {
            time=source.time; value=source.value; in_tangent=source.inTangent; out_tangent=source.outTangent;
            in_weight=source.inWeight; out_weight=source.outWeight; weighted_mode=(int)source.weightedMode;
        }
    }
    [Serializable] public class Curve
    {
        public string owner, name;
        public Key[] keys;
        public int pre_wrap, post_wrap;
        public float[] inputs, values;
    }
    [Serializable] public class Scalar { public string name, value; }
    [Serializable] public class PartIdentity { public string path; public int category, item; }
    [Serializable] public class AssemblyIdentity
    {
        public float mass;
        public Vector3 center_of_mass;
        public PartIdentity[] parts;
        public Scalar[] engine_parameters;
        public Key[] torque_keys;
        public string fuel_recipe;
        public WheelParameters[] wheels;
    }
    [Serializable] public class PoseInput { public Vector3 position; public Quaternion rotation; }
    [Serializable] public class Condition { public string path, material_type; public int index, state, state2, state3; public Color color; }
    [Serializable] public class Tank { public int index; public string path, fluid_json; }
    [Serializable] public class AssemblySetup { public Condition[] conditions; public Tank[] tanks; }
    [Serializable] public class Initial
    {
        public string car_path, engine_path, driver_seat_path, scene, random_state, profile_path, setup_json;
        public int car_type;
        public float mass, fixed_dt, engine_rpm, engine_temperature;
        public Vector3 center_of_mass, position, velocity, angular_velocity, gravity, inertia_tensor;
        public Quaternion rotation, inertia_tensor_rotation;
        public string fuel_json, oil_json, coolant_json;
        public bool auto_clutch, auto_shift, ignition, running, stored_handbrake;
        public int selected_gear;
        public G11Controls.Mode control_mode;
        public Wheel[] wheel_state;
        public Condition[] conditions;
        public AssemblySetup assembly_setup;
        public AssemblyIdentity assembly_identity;
        public PoseInput input_pose, spawn_pose;
        public G11Driver.Input driver_input;
        public G11Cargo.Input cargo_input;
        public G11AngularProbe.BodyInput angular_body_input;public G11AngularProbe.BodyInput contact_body_input;
        public WheelParameters[] wheels;
        public WheelParameters[] source_control_wheels;
    }
    [Serializable] public class SetupBoundary
    {
        public string phase; public int frame; public float time, fixed_time, delta_time; public bool kinematic;
        public Vector3 inertia_tensor; public Quaternion rotation,inertia_tensor_rotation;
    }
    [Serializable] public class Result
    {
        public int schema_version=1;
        public string runtime="original-unity-player", scenario, probe_kind, error;
        public bool completed, menu_ready, lada_ready, control_capture_active, isolated_wheel_probe;
        public int physics_steps;
        public string units="metres; seconds; kilograms; degrees; rpm; newtons; newton metres; Unity WheelHit dimensionless slip";
        public Initial initial, setup_state, observed_before_canonical;
        public AssemblyIdentity authored_template, scene_assembly, scene_before_setup, prepared_assembly;
        public bool target_held_before_activation;
        public Condition[] held_conditions;
        public AssemblyIdentity held_assembly;
        public string template_source="level1/64898.StartCar02 -> level1/206 -> level1/17687 carscript level1/77839";
        public Sample[] samples;
        public G11AngularProbe.Report angular;public G11ContactProbe.Report contact;
        public G11ContactProbe.Readout contact_configure_readout;
        public G11Geometry.Activation[] wheel_activation_events;
        public List<G11Geometry.OwnershipCheckpoint> wheel_ownership=G11Geometry.ownershipCheckpoints;
        public List<SetupBoundary> setup_boundaries=new List<SetupBoundary>();
        public List<G11Geometry.Snapshot> geometry=G11Geometry.snapshots;
        public List<Curve> curves=new List<Curve>();
        public List<string> setup_changes=new List<string>();
        public G11Driver.Report driver=G11Driver.report;
        public G11Lifecycle.Report lifecycle=G11Lifecycle.report;
        public G11BodyTrace.Report body_trace=G11BodyTrace.report;
        public G11Cargo.Report cargo_input=G11Cargo.report;
        public string[] unavailable={"WheelCollider internal angular inertia", "PhysX internal contact solver impulses", "Independent native engine, service, and parking brake torque decomposition"};
    }
    static readonly HashSet<partslotscript> completedMounts=new HashSet<partslotscript>();
    static readonly HashSet<mountStuff> completedStorage=new HashSet<mountStuff>();
    static AssemblyIdentity assemblyIdentity;
    static bool pendingArm,pendingDriver;
    static string preparedGeometry;
    static G11Capture instance;
    static Scenario scenario;
    static readonly Result result=new Result();
    static carscript car;
    static AssemblySetup assemblySetup;
    static seatscript driverSeat;
    static Rigidbody body;
    static WheelCollider[] wheels, sourceWheels;
    static PoseInput inputPose, spawnPose;
    static Sample[] buffer;
    static int observedPhysicsStep;
    static int controlRevision;
    static int used, sequence, step, startFrame, lastInput, lastEngine, lastCar, lastRpm;
    static bool active, finished, sourceContactAvailable;
    static WheelHit sourceContact;
    static readonly object bufferGate=new object();
    static readonly Dictionary<int,string> surfacePaths=new Dictionary<int,string>();
    static Timer timeoutTimer;
    static string Output { get { return Environment.GetEnvironmentVariable("G11_RUN_ROOT"); } }
    public static string ProfilePath() { return Environment.GetEnvironmentVariable("G11_PROFILE_ROOT"); }
    static string PathOf(Transform transform)
    {
        return transform.parent==null?transform.name:PathOf(transform.parent)+"/"+transform.name;
    }
    void Awake()
    {
        instance=this; DontDestroyOnLoad(gameObject);
        Application.SetStackTraceLogType(LogType.Error,StackTraceLogType.Full);
        Application.logMessageReceived+=CaptureLog;
        // Keep partial samples if the main thread stalls before the launcher timeout.
        timeoutTimer=new Timer(delegate(object state)
        {
            lock(bufferGate)
            {
                if(finished) return;
                result.completed=false; result.error="170-second observer deadline before launcher timeout";
                Flush();
            }
        },null,170000,Timeout.Infinite);
    }
    static void CaptureLog(string message,string stack,LogType type)
    {
        if(!finished && (type==LogType.Exception || type==LogType.Error) && String.IsNullOrEmpty(result.error))
            result.error="Original runtime error before capture flush: "+message+"\n"+stack;
    }
    public static void AudioCreated(GameObject value,AudioClip clip)
    { try { G11Lifecycle.Created(value,clip); } catch(Exception error) { Fail(error); } }
    public static void AudioDestroy(UnityEngine.Object value,float delay)
    {
        try { G11Lifecycle.DestroyRequested(value,delay); } catch(Exception error) { Fail(error); }
        UnityEngine.Object.Destroy(value,delay);
    }
    public static bool DriverRay(Vector3 origin,Vector3 direction,out RaycastHit info,float distance,int mask)
    {
        bool hit=Physics.Raycast(origin,direction,out info,distance,mask);
        G11Lifecycle.RayObserved(origin,direction,distance,mask,hit,info);return hit;
    }
    public static void Observe(object owner,string phase)
    {
        try
        {
            G11Lifecycle.OwnerBoundary(owner,phase);
            if(phase.StartsWith("setup.",StringComparison.Ordinal)) return;
            if(phase=="storage.ready")
            {
                completedStorage.Add((mountStuff)owner);
                return;
            }
            if(phase=="slot.ready")
            {
                completedMounts.Add((partslotscript)owner);
                return;
            }
            if(phase=="car.ready")
            {
                var ready=(carscript)owner;
                if(ready.carType==itemdatabase.CarType.car02 && ready.RB!=null)
                {
                    G11BodyTrace.Set(ready.RB,true,"System.Void G11Capture::Observe(System.Object,System.String)",-1,"observer.car.ready");
                    if(ready==car) G11Geometry.HoldWheels();
                    else foreach(var wheel in ready.allWH) wheel.enabled=false;
                    result.setup_changes.Add("Hold source Lada body and disable native wheels after original car Start");
                }
                return;
            }
            if(phase=="settings.entry")
            {
                settingsscript.s.S.BMirrors=false;
                result.setup_changes.Add("Scratch visual input: original mirrors-off setting before SettingsRefreshed; driving settings unchanged");
                return;
            }
            if(phase=="menu.exit")
            {
                if(instance!=null) throw new InvalidOperationException("Duplicate original menu initialization");
                scenario=JsonUtility.FromJson<Scenario>(File.ReadAllText(Path.Combine(Output,"scenario.json")));
                if(scenario.probe_kind=="single_wheel_contact")
                {
                    result.contact_configure_readout=G11ContactProbe.readout;
                    G11ContactProbe.ObserveAfterFromJson(scenario.contact_body_input_from_source,scenario.contact_body_input,
                        new G11ContactProbe.SetupMark{frame=Time.frameCount,physics_step=ObservedPhysicsStep,input_revision=InputRevision});
                }
                result.scenario=scenario.id; result.probe_kind=scenario.probe_kind;
                new GameObject("G11NativeObserver").AddComponent<G11Capture>();
                instance.StartCoroutine(instance.Load((mainmenuscript)owner));
                return;
            }
            if(phase=="world.entry")
            {
                var main=(mainscript)owner;
                var start=main.terrainGenerationSettings.StartCar02;
                if(start.activeInHierarchy) throw new InvalidOperationException("Authored StartCar02 already activated before pristine capture");
                var authored=start.GetComponentsInChildren<carscript>(true).Single(c=>c.carType==itemdatabase.CarType.car02);
                result.authored_template=DescribeAssembly(authored);
                car=authored; body=car.RB;
                G11Geometry.Initialize(car);G11BodyTrace.Bind(car);G11Lifecycle.Begin();
                G11Geometry.Capture("world.entry",observedPhysicsStep,"world.entry",completedStorage);
                G11BodyTrace.HoldOriginal();
                G11Geometry.HoldWheels();
                result.target_held_before_activation=true;
                result.setup_changes.Add("Use the actual authored StartCar02 car in each fresh source process; hold body and disable every native wheel before original activation");
                return;
            }
            if(phase=="world.exit")
            {
                instance.StartCoroutine(instance.Initialize((mainscript)owner));
                return;
            }
            if(finished) return;
            if(phase.StartsWith("driver.",StringComparison.Ordinal))
            { G11Driver.Observe(owner,phase,observedPhysicsStep);return; }
            if(phase=="input.exit") controlRevision++;
            if(!active) return;
            if(owner is carscript && owner!=car) return;
            if(owner is enginescript && owner!=car.Engine) return;
            if(startFrame<0) startFrame=Time.frameCount;
            if(phase=="input.entry")
            {
                lastInput=sequence+1;
            }
            if(phase=="car.entry") lastCar=sequence+1;
            if(phase=="engine.entry") lastEngine=sequence+1;
            if(phase=="rpm.exit") lastRpm=sequence+1;
            Record(phase);
            if(phase=="control.exit") result.control_capture_active=true;
        }
        catch(Exception error) { Fail(error); }
    }
    IEnumerator Load(mainmenuscript menu)
    {
        yield return null; yield return null;
        try
        {
            if(!menu.started || menu.DFMS==null || settingsscript.s==null)
                throw new InvalidOperationException("Original menu did not initialize");
            result.menu_ready=true;
            // A declared input seed; the source random implementation stays intact.
            UnityEngine.Random.InitState(scenario.setup.seed);
            menu.seed=scenario.setup.seed; menu.DFMS.seed=scenario.setup.seed;
            menu.DFMS.startcar=itemdatabase.CarType.car02;
            menu.PressedLoadScene("SceneNewRandom");
        }
        catch(Exception error) { Fail(error); }
    }
    IEnumerator Initialize(mainscript main)
    {
        // Each fresh process supplies the original authored scene instance, held before activation.
        for(int frame=0;frame<600;frame++)
        {
            yield return null;
            if(SourceAssemblyReady(car) && main.player!=null && main.player.mind) break;
        }
        try
        {
            if(!result.target_held_before_activation || !SourceAssemblyReady(car) || !main.player.mind)
            {
                G11Geometry.Capture("readiness-timeout",observedPhysicsStep,"Initialize bounded timeout",completedStorage);
                throw new InvalidOperationException("Original selected Lada, source storage, and player did not initialize; physical readiness: "+G11Geometry.ReadinessFailure()+"; source readiness: "+SourceReadinessFailure(car));
            }
            G11Geometry.Capture("source-physical-ready",observedPhysicsStep,"Initialize source readiness",completedStorage);
            if(scenario.has_setup_state && (scenario.setup_state==null || scenario.setup_state.cargo_input==null))
                throw new InvalidOperationException("Canonical source cargo input is missing");
            G11Geometry.ApplyCargoInput(scenario.has_setup_state?scenario.setup_state.cargo_input:null,observedPhysicsStep,controlRevision,completedStorage);
            main.PressedContinue();
            body=car.RB;
            G11BodyTrace.Set(body,true,"System.Collections.IEnumerator G11Capture::Initialize(mainscript)",-1,"observer.Initialize");
            sourceWheels=car.allWH.ToArray(); wheels=sourceWheels;
            G11Geometry.BindNativeWheelRoles();
            if(wheels.Any(w=>w.enabled)) throw new InvalidOperationException("Original native wheels enabled before setup completion");
            spawnPose=scenario.has_setup_state?scenario.setup_state.spawn_pose:
                new PoseInput{position=car.transform.position,rotation=car.transform.rotation};
            result.scene_before_setup=DescribeAssembly(car);
            assemblySetup=scenario.has_setup_state?scenario.setup_state.assembly_setup:ReadAssemblySetup(car);
            ApplyAssemblySetup(car,assemblySetup);
            result.scene_assembly=DescribeAssembly(car);
            assemblyIdentity=scenario.has_setup_state?scenario.setup_state.assembly_identity:result.scene_assembly;
            RequireAssembly(assemblyIdentity,result.scene_assembly,"initialized original authored garage Lada across repeats");
            ApplySetup(main);
            foreach(var graphic in car.GetComponentsInChildren<wheelgraphicsscript>()) graphic.WheelSize();
            driverSeat=car.GetComponentsInChildren<seatscript>().Single(seat=>seat.driverSeat0 && seat.Car==car);
            G11Driver.Retain(main.player,car,driverSeat);
            main.player.GetIn(driverSeat);
            if(!car.driving || !car.isPlayerDriving || main.player.Car!=car || driverSeat.Car!=car)
                throw new InvalidOperationException("Original authored Lada driver ownership failed");
            if(UnityEngine.Object.FindObjectsOfType<carscript>().Count(target=>target.carType==itemdatabase.CarType.car02)!=1)
                throw new InvalidOperationException("Original authored Lada is not the sole active target");
            result.lada_ready=true;
            result.setup_changes.Add("Sample and control only the original authored Lada, its engine, and its original driver seat");
            foreach(var wheel in wheels) if(wheel.GetGroundHit(out sourceContact)) { sourceContactAvailable=true; break; }
            if(scenario.probe_kind=="wheel_collider") CreateIsolatedProbe();
            if(scenario.probe_kind=="wheel_angular") CreateAngularProbe();
            if(scenario.probe_kind=="single_wheel_contact") CreateContactProbe();
            foreach(var wheel in wheels) wheel.enabled=false;
            if(scenario.has_setup_state) inputPose=scenario.setup_state.input_pose;
            else
            {
                if(!G11AngularProbe.Active && !G11ContactProbe.Active) body.mass*=scenario.setup.load_multiplier;
                if(scenario.setup.surface=="nearby-source-slope") PlaceOnSourceSlope();
                if(scenario.setup.height=="one-source-suspension-drop") body.position+=Vector3.up*wheels[0].suspensionDistance;
                if(scenario.setup.height=="wheels-clear") body.position+=Vector3.up*(wheels[0].radius*4+wheels[0].suspensionDistance);
                inputPose=new PoseInput{position=body.transform.position,rotation=body.transform.rotation};
            }
            G11Lifecycle.OwnersCompleted(SourceAssemblyReady(car),"SourceAssemblyReady: original slot SpawnPart exits, cargo StartFreeze terminal exits, crafted/part started flags, source physical readiness, explicit assembly and setup completed");
            pendingDriver=true;
            result.setup_changes.Add("Observe actual source M.Audio clone references through original Destroy and Unity-null; close setup owners before rest readiness; observe original player forward, ray, Look and midpoint boundaries without pose rewrites");
            result.setup_changes.Add("Retain the original driver Rigidbody and its exact attached collider transfer set before original GetIn; observe native destruction and source Update exits separately");
            result.setup_changes.Add("Observer setup once: Tb/Th/THeadDir/THeadBob/THeadLean/TdefStandPos/TStandHeadPos/TProneHeadPos localPosition/localRotation/localScale; target positions derive from source TdefStandPos; FxRot/FyRot/dt/duckValue/Bduck/lastDucked/standLean/PRBV/parentRigidbody; anim.VCarV/VHeadV/VHeadBob authored zeros; LOOKX/LOOKY/scroll and input lookX/lookY/scroll/lean/leanDown/leanUp/recenter/recenterDown/recenterUp/keyLooking/duck/duckDown/duckUp neutral; source Update owns physical shapes, look point and piss pose");

        }
        catch(Exception error) { Fail(error); }
    }
    static void PrepareHeld()
    {
        // Record pre-solve inputs after the real source seated Update completes.
        result.prepared_assembly=DescribeAssembly(car);
        result.observed_before_canonical=InitialState();
        result.setup_state=scenario.has_setup_state?scenario.setup_state:result.observed_before_canonical;
        result.setup_state.profile_path=null;
        if(!Physics.autoSimulation) throw new InvalidOperationException("Original native auto simulation is inactive");
        CaptureCurves();
        buffer=new Sample[(scenario.duration_frames+1)*256];
        for(int index=0;index<buffer.Length;index++)
        { buffer[index].wheels=new Wheel[wheels.Length];buffer[index].angular_state=G11AngularProbe.Active?new G11AngularProbe.State():null;buffer[index].contact_state=G11ContactProbe.Active?new G11ContactProbe.State():null;buffer[index].native_wheels=new G11Geometry.NativeWheel[G11Geometry.nativeWheels.Length]; }
        RecordSetupBoundary("prepared-held");
        preparedGeometry=G11Geometry.Inputs(G11Geometry.snapshots[G11Geometry.snapshots.Count-1]);
        G11Driver.Prepared();pendingDriver=false;pendingArm=true;
        result.setup_changes.Add("Require unchanged strict physical geometry across the next completed original seated Update while held; retain normal callbacks and apply no driver correction after arming");
    }
    static bool SourceAssemblyReady(carscript target)
    {
        if(target==null || target.RB==null || target.Engine==null || target.allWH.Count!=4 ||
           !target.Engine.transform.IsChildOf(target.transform)) return false;
        foreach(var slot in target.GetComponentsInChildren<partslotscript>(true))
        {
            if(slot.isActiveAndEnabled && !completedMounts.Contains(slot)) return false;
            if(slot.part!=null && (!slot.part.transform.IsChildOf(target.transform) ||
               slot.part.slot!=slot || !slot.part.p.crafted || !slot.part.tosaveitem.started)) return false;
        }
        if(target.GetComponentsInChildren<mountStuff>(true).Any(m=>m.isActiveAndEnabled && !completedStorage.Contains(m))) return false;
        return target.GetComponentsInChildren<partconditionscript>(true).All(p=>p.started) && G11Geometry.PhysicalReady();
    }
    static string SourceReadinessFailure(carscript target)
    {
        var errors=new List<string>();
        if(target==null) return "selected car missing";
        if(target.RB==null) errors.Add("source car.RB missing");
        if(target.Engine==null) errors.Add("source Engine missing");
        if(target.allWH.Count!=4) errors.Add("source allWH count="+target.allWH.Count);
        foreach(var slot in target.GetComponentsInChildren<partslotscript>(true))
        {
            if(slot.isActiveAndEnabled && !completedMounts.Contains(slot)) errors.Add("source slot callback pending "+G11Geometry.Key(slot));
            if(slot.part!=null && (!slot.part.transform.IsChildOf(target.transform) || slot.part.slot!=slot || !slot.part.p.crafted || !slot.part.tosaveitem.started))
                errors.Add("source slot ancestry/craft/start pending "+G11Geometry.Key(slot));
        }
        foreach(var mount in target.GetComponentsInChildren<mountStuff>(true))
            if(mount.isActiveAndEnabled && !completedStorage.Contains(mount)) errors.Add("source storage callback pending "+G11Geometry.Key(mount));
        foreach(var condition in target.GetComponentsInChildren<partconditionscript>(true))
            if(!condition.started) errors.Add("source condition startup pending "+G11Geometry.Key(condition));
        return String.Join("; ",errors.ToArray());
    }
    static string ScalarValue(object value)
    {
        return value is float?((float)value).ToString("R",CultureInfo.InvariantCulture):
            Convert.ToString(value,CultureInfo.InvariantCulture);
    }
    static string RelativePath(Transform root,Transform value)
    {
        if(value==root) return ".";
        string path=value.name;
        while(value.parent!=root)
        {
            value=value.parent;
            if(value==null) throw new InvalidOperationException("Source assembly member is outside its car hierarchy");
            path=value.name+"/"+path;
        }
        return path;
    }
    static AssemblyIdentity DescribeAssembly(carscript target)
    {
        var order=target.WHDriven.Concat(target.WHSteer).Concat(target.WHInvSteer)
            .Concat(target.WHBrakeOnly).Concat(target.WHHandBrake).Distinct().ToArray();
        if(order.Length!=4) throw new InvalidOperationException("Authored Lada must own four native wheel profiles");
        var engine=target.GetComponentsInChildren<enginescript>(true).Single();
        string[] parameters={"startChance","starterRpm","starterRpmLerp","alapRpm","maxRpm","shutdownRpm","maxNm",
            "consumptionM","OilConsumptionModifier","motorBrakeModifier","rpmChangeModifier","engineStartTimeModifier",
            "engineHeatGainMin","engineHeatGainMax","engineHeatLoss","engineHeatLossSpeedModifier","minOptimalTemp1",
            "maxOptimalTemp1","minOptimalTemp2","maxOptimalTemp2","twostroke","maxRpmDifference","modifier","Oilfluid",
            "noOverHeat","kickstart","autoShiftClutchRpm","autoShiftClutchRpmRange"};
        return new AssemblyIdentity{mass=target.RB.mass,center_of_mass=target.RB.centerOfMass,
            parts=target.GetComponentsInChildren<tosaveitemscript>(true).Select(p=>new PartIdentity{
                path=RelativePath(target.transform,p.transform),category=p.category,item=p.id}).ToArray(),
            engine_parameters=parameters.Select(name=>new Scalar{name=name,
                value=ScalarValue(typeof(enginescript).GetField(name).GetValue(engine))}).ToArray(),
            torque_keys=engine.torqueCurve.keys.Select(key=>new Key(key)).ToArray(),fuel_recipe=JsonUtility.ToJson(engine.FuelConsumption),
            wheels=order.Select(w=>new WheelParameters{path=RelativePath(target.transform,w.transform),local_position=w.transform.localPosition,
                local_rotation=w.transform.localRotation,center=w.center,radius=w.radius,mass=w.mass,damping=w.wheelDampingRate,
                suspension_distance=w.suspensionDistance,force_application_distance=w.forceAppPointDistance,
                spring_n_per_m=w.suspensionSpring.spring,damper_n_s_per_m=w.suspensionSpring.damper,spring_target=w.suspensionSpring.targetPosition,
                forward=new Friction(w.forwardFriction),sideways=new Friction(w.sidewaysFriction),driven=target.WHDriven.Contains(w),
                steer=target.WHSteer.Contains(w),service_brake=target.WHBrakeOnly.Contains(w),handbrake=target.WHHandBrake.Contains(w)}).ToArray()};
    }
    static void RequireAssembly(AssemblyIdentity expected,AssemblyIdentity observed,string phase)
    {
        if(JsonUtility.ToJson(expected)!=JsonUtility.ToJson(observed))
            throw new InvalidOperationException("Source assembly membership, mass, engine, or native wheel profiles differ: "+phase);
    }
    static void ApplySetup(mainscript main)
    {
        main.crsrLocked=true;
        settingsscript.s.S.BAutoClutch=scenario.setup.auto_clutch;
        settingsscript.s.S.BAutoShift=scenario.setup.auto_shift;
        settingsscript.s.S.BForceManual=false;
        settingsscript.s.S.CBenableGas=false; settingsscript.s.S.CBenableBrake=false;
        settingsscript.s.S.CBenableClutch=false; settingsscript.s.S.CBenableSteer=false;
        settingsscript.s.S.CBenableHandBrake=false; settingsscript.s.S.CBHShifter=false;
        inputscript.i.mouseDrive=false;
        result.setup_changes.Add("Controls and assists: "+JsonUtility.ToJson(scenario.setup));
        if(scenario.setup.engine!="source-startup" || scenario.setup.fluids!="source-startup")
            throw new InvalidOperationException("Unsupported engine or fluid setup");
        var slots=car.GetComponentsInChildren<partslotscript>().Where(s=>s.part!=null && s.part.p!=null && s.part.p.wheel!=null).ToArray();
        foreach(var slot in slots)
        {
            var rim=slot.part.p.wheel;
            if(scenario.setup.support=="full-tire" || scenario.setup.support=="worn-tire")
            {
                if(rim.gumi.part==null || rim.gumi.part.p.gumi==null) throw new InvalidOperationException("Source tire is absent");
                rim.gumi.part.p.gumi.pc.state=scenario.setup.support=="full-tire"?0:4;
                rim.gumi.part.p.gumi.Refresh(); rim.Refresh();
            }
            else if(scenario.setup.support=="bare-rim")
            {
                var tire=rim.gumi.part;
                if(tire==null || !rim.gumi.UnCraft(true)) throw new InvalidOperationException("Source tire removal failed");
                tire.gameObject.SetActive(false); rim.Refresh();
            }
            else if(scenario.setup.support=="hub")
            {
                var part=slot.part;
                if(!slot.UnCraft(true)) throw new InvalidOperationException("Source rim removal failed");
                part.gameObject.SetActive(false);
            }
            else throw new InvalidOperationException("Unknown support setup");
        }
        for(int index=0;index<scenario.setup.removed_driven_rims;index++)
        {
            var slot=car.DrivenWheelSlots[index]; var part=slot.part;
            if(part==null || !slot.UnCraft(true)) throw new InvalidOperationException("Driven rim removal failed");
            part.gameObject.SetActive(false);
        }
    }
    static Condition[] ReadConditions(carscript target)
    {
        return target.GetComponentsInChildren<partconditionscript>(true).Select((c,index)=>new Condition{
            index=index,path=RelativePath(target.transform,c.transform),state=c.state,state2=c.state2,state3=c.state3,
            color=c.color,material_type=c.tipus}).ToArray();
    }
    static AssemblySetup ReadAssemblySetup(carscript target)
    {
        return new AssemblySetup{conditions=ReadConditions(target),
            tanks=target.GetComponentsInChildren<tankscript>(true).Select((tank,index)=>new Tank{
                index=index,path=RelativePath(target.transform,tank.transform),fluid_json=JsonUtility.ToJson(tank.F)}).ToArray()};
    }
    static void ApplyConditions(carscript target,Condition[] recorded)
    {
        var conditions=target.GetComponentsInChildren<partconditionscript>(true);
        if(conditions.Length!=recorded.Length) throw new InvalidOperationException("Canonical source condition component count differs: expected "+recorded.Length+" observed "+conditions.Length);
        foreach(var condition in recorded)
        {
            var component=conditions[condition.index];
            if(RelativePath(target.transform,component.transform)!=condition.path)
                throw new InvalidOperationException("Canonical source condition component order differs");
            if(component.tipus!=condition.material_type)
                throw new InvalidOperationException("Canonical source condition material type differs: "+condition.path);
            component.state2=condition.state2; component.state3=condition.state3;
            component.Refresh(condition.state,condition.color);
        }
        if(JsonUtility.ToJson(new AssemblySetup{conditions=ReadConditions(target)})!=
           JsonUtility.ToJson(new AssemblySetup{conditions=recorded}))
            throw new InvalidOperationException("Source condition Refresh did not retain the canonical record");
    }
    static void RefreshSourceWheels(carscript target)
    {
        foreach(var tire in target.GetComponentsInChildren<gumiscript>()) tire.Refresh();
        foreach(var rim in target.GetComponentsInChildren<wheelscript>()) rim.Refresh();
        foreach(var graphic in target.GetComponentsInChildren<wheelgraphicsscript>()) graphic.WheelSize();
    }
    static void ApplyAssemblySetup(carscript target,AssemblySetup recorded)
    {
        ApplyConditions(target,recorded.conditions);
        var tanks=target.GetComponentsInChildren<tankscript>(true);
        if(tanks.Length!=recorded.tanks.Length) throw new InvalidOperationException("Canonical source tank component count differs");
        foreach(var tank in recorded.tanks)
        {
            if(RelativePath(target.transform,tanks[tank.index].transform)!=tank.path)
                throw new InvalidOperationException("Canonical source tank component order differs");
            JsonUtility.FromJsonOverwrite(tank.fluid_json,tanks[tank.index].F);
        }
        RefreshSourceWheels(target);
        target.GetComponent<massScript>().Upd();
        result.setup_changes.Add("Apply indexed source condition Refresh(state,color), tank records, gumiscript.Refresh, wheelscript.Refresh, wheelgraphicsscript.WheelSize, massScript.Upd before the initialized assembly check");
    }
    static void ApplyCanonical(Initial initial)
    {
        if(!G11AngularProbe.Active && !G11ContactProbe.Active) {
            body.mass=initial.mass;
            body.centerOfMass=initial.center_of_mass;
        }
        car.Engine.rpm=initial.engine_rpm; car.Engine.temp=initial.engine_temperature;
        car.ignition=initial.ignition; car.Engine.running=initial.running; car.BhandBrake=initial.stored_handbrake;
        car.ShiftTo(initial.selected_gear,true);
        if(car.Tank!=null) JsonUtility.FromJsonOverwrite(initial.fuel_json,car.Tank.F);
        if(car.Engine.T!=null) JsonUtility.FromJsonOverwrite(initial.oil_json,car.Engine.T.F);
        if(car.coolant!=null) JsonUtility.FromJsonOverwrite(initial.coolant_json,car.coolant.coolant.F);
        ApplyConditions(car,initial.conditions);
        RefreshSourceWheels(car);
        Physics.SyncTransforms();
        if(body==car.RB)
        {
            RecordSetupBoundary("before-inertia-reset");
            body.ResetInertiaTensor();
            RecordSetupBoundary("after-inertia-reset");
        }
        result.setup_changes.Add("Recompute original body inertia from synchronized source colliders with ResetInertiaTensor; retain native automatic inertia updates; never set a manual source body tensor");
        body.position=inputPose.position; body.rotation=inputPose.rotation;
        if(!G11AngularProbe.Active && !G11ContactProbe.Active) { body.velocity=body.transform.TransformDirection(scenario.setup.velocity_mps);body.angularVelocity=Vector3.zero; }
        UnityEngine.Random.state=JsonUtility.FromJson<UnityEngine.Random.State>(initial.random_state);
        result.setup_changes.Add("Apply one canonical pre-solve body, engine, fluid, condition, gear, and random record to every repeat");
    }
    static void PlaceOnSourceSlope()
    {
        // Select actual nearby terrain by a declared search. Never create a slope for assembled evidence.
        Vector3 origin=body.position;
        for(int radius=5;radius<=40;radius+=5)
        for(int direction=0;direction<8;direction++)
        {
            float angle=direction*Mathf.PI*.25f;
            Vector3 point=origin+new Vector3(Mathf.Cos(angle)*radius,100f,Mathf.Sin(angle)*radius);
            RaycastHit hit;
            if(!Physics.Raycast(point,Vector3.down,out hit,200f)) continue;
            if(hit.collider.attachedRigidbody!=null || hit.collider.isTrigger) continue;
            float slope=Vector3.Angle(hit.normal,Vector3.up);
            if(slope<2f || slope>20f) continue;
            body.position=hit.point+hit.normal*(wheels[0].radius+wheels[0].suspensionDistance);
            body.rotation=Quaternion.LookRotation(Vector3.ProjectOnPlane(body.transform.forward,hit.normal),hit.normal);
            result.setup_changes.Add("Measured nearby source slope: "+PathOf(hit.collider.transform)+
                " point="+hit.point+" normal="+hit.normal+" degrees="+slope);
            return;
        }
        throw new InvalidOperationException("No qualifying nearby source slope found");
    }
    static Initial InitialState()
    {
        return new Initial { car_path=PathOf(car.transform), engine_path=PathOf(car.Engine.transform), driver_seat_path=PathOf(driverSeat.transform),
            scene=UnityEngine.SceneManagement.SceneManager.GetActiveScene().name, car_type=(int)car.carType,
            random_state=JsonUtility.ToJson(UnityEngine.Random.state), profile_path=ProfilePath(),
            setup_json=JsonUtility.ToJson(scenario.setup), angular_body_input=G11AngularProbe.Active?G11AngularProbe.report.body_input:null, contact_body_input=G11ContactProbe.Active?G11ContactProbe.report.body_input:null, cargo_input=G11Cargo.report.input, driver_input=G11Driver.report.input, input_pose=inputPose, spawn_pose=spawnPose, mass=body.mass, center_of_mass=body.centerOfMass,
            inertia_tensor=body.inertiaTensor,inertia_tensor_rotation=body.inertiaTensorRotation,
            position=body.position, rotation=body.rotation, velocity=body.velocity, angular_velocity=body.angularVelocity,
            fixed_dt=Time.fixedDeltaTime, gravity=Physics.gravity, engine_rpm=car.Engine.rpm, engine_temperature=car.Engine.temp,
            auto_clutch=settingsscript.s.S.BAutoClutch,auto_shift=settingsscript.s.S.BAutoShift,control_mode=G11Controls.ReadMode(car),
            ignition=car.ignition,running=car.Engine.running,stored_handbrake=car.BhandBrake,selected_gear=car.gear,
            wheel_state=wheels.Select(w=>ReadWheel(w)).ToArray(),
            source_control_wheels=(G11AngularProbe.Active || G11ContactProbe.Active)?DescribeAssembly(car).wheels:null,
            conditions=ReadConditions(car),assembly_setup=assemblySetup,assembly_identity=assemblyIdentity,
            fuel_json=car.Tank==null?"unavailable":JsonUtility.ToJson(car.Tank.F),
            oil_json=car.Engine.T==null?"unavailable":JsonUtility.ToJson(car.Engine.T.F),
            coolant_json=car.coolant==null?"unavailable":JsonUtility.ToJson(car.coolant.coolant.F),
            wheels=wheels.Select((w,index)=>new WheelParameters {path=PathOf(w.transform),local_position=w.transform.localPosition,
                local_rotation=w.transform.localRotation,center=w.center,radius=w.radius,mass=w.mass,damping=w.wheelDampingRate,
                suspension_distance=w.suspensionDistance,force_application_distance=w.forceAppPointDistance,
                spring_n_per_m=w.suspensionSpring.spring,damper_n_s_per_m=w.suspensionSpring.damper,
                spring_target=w.suspensionSpring.targetPosition,forward=new Friction(w.forwardFriction),sideways=new Friction(w.sidewaysFriction),
                driven=!G11AngularProbe.Active && !G11ContactProbe.Active && car.WHDriven.Contains(sourceWheels[index]),steer=!G11AngularProbe.Active && !G11ContactProbe.Active && car.WHSteer.Contains(sourceWheels[index]),service_brake=!G11AngularProbe.Active && !G11ContactProbe.Active && car.WHBrakeOnly.Contains(sourceWheels[index]),
                handbrake=!G11AngularProbe.Active && !G11ContactProbe.Active && car.WHHandBrake.Contains(sourceWheels[index])}).ToArray() };
    }
    static void CaptureCurves()
    {
        var fields=typeof(carscript).GetFields().Where(f=>f.FieldType==typeof(AnimationCurve));
        foreach(var field in fields) CaptureCurve("carscript",field.Name,(AnimationCurve)field.GetValue(car),null);
        CaptureCurve("enginescript","torqueCurve",car.Engine.torqueCurve,scenario.engine_torque_holdout_inputs);
    }
    static void CaptureCurve(string owner,string name,AnimationCurve curve,float[] appendedInputs)
    {
        if(curve==null || curve.length==0) return;
        var keys=curve.keys;
        // The outside values use one source key interval, rather than a recreated extrapolation formula.
        float interval=keys.Length>1?keys[keys.Length-1].time-keys[0].time:1f;
        var inputs=new List<float>{0f,keys[0].time-interval};
        foreach(var key in keys) inputs.Add(key.time);
        inputs.Add(keys[keys.Length-1].time+interval);
        for(int index=1;index<keys.Length;index++)
        {
            float span=keys[index].time-keys[index-1].time;
            for(int quarter=1;quarter<=3;quarter++) inputs.Add(keys[index-1].time+span*quarter*.25f);
        }
        if(appendedInputs!=null) inputs.AddRange(appendedInputs);
        result.curves.Add(new Curve{owner=owner,name=name,keys=keys.Select(k=>new Key(k)).ToArray(),pre_wrap=(int)curve.preWrapMode,
            post_wrap=(int)curve.postWrapMode,inputs=inputs.ToArray(),values=inputs.Select(curve.Evaluate).ToArray()});
    }
    static Rigidbody CreateHeldProbeBody(GameObject probe)
    {
        var copied=probe.AddComponent<Rigidbody>();
        G11BodyTrace.HoldCopy(copied);
        return copied;
    }
    static void CreateContactProbe()
    {
        var source=G11Geometry.nativeWheels.Single(w=>G11Geometry.Key(w)==scenario.contact.source_wheel);
        var original=body;var probe=new GameObject("G11SingleWheelContactProbe");
        var copied=CreateHeldProbeBody(probe);
        wheels=new[]{G11ContactProbe.Configure(scenario.contact,source,original,probe,copied,scenario.contact_body_input_from_source,scenario.contact_body_input,
            new G11ContactProbe.SetupMark{frame=Time.frameCount,physics_step=ObservedPhysicsStep,input_revision=InputRevision})};
        G11Geometry.IsolateSourceCar();body=copied;result.isolated_wheel_probe=true;result.contact=G11ContactProbe.report;
    }
    static void CreateAngularProbe()
    {
        var source=G11Geometry.nativeWheels.Single(w=>G11Geometry.Key(w)==scenario.angular.source_wheel);
        var original=body;
        var probe=new GameObject("G11AirborneAngularProbe");
        probe.transform.position=original.position+scenario.angular.rig.offset_m;probe.transform.rotation=original.rotation;
        var copied=CreateHeldProbeBody(probe);
        wheels=new[]{G11AngularProbe.Configure(scenario.angular,source,original,probe,copied,scenario.has_setup_state,scenario.has_setup_state?scenario.setup_state.angular_body_input:null)};
        G11Geometry.IsolateSourceCar();body=copied;result.isolated_wheel_probe=true;result.angular=G11AngularProbe.report;
        result.setup_changes.Add("One copied native wheel; no copied ground or body collider; fixed-step torque inputs; original held by reviewed owner");
    }
    static void CreateIsolatedProbe()
    {
        // Copy live source WheelCollider properties. No tire or engine formula runs in this probe.
        var original=wheels;
        var offset=Vector3.right*10000f;
        var probe=new GameObject("G11IsolatedNativeWheelProbe");
        probe.transform.position=body.position+offset; probe.transform.rotation=body.rotation;
        var rb=CreateHeldProbeBody(probe); rb.mass=body.mass; rb.centerOfMass=body.centerOfMass;
        rb.drag=body.drag; rb.angularDrag=body.angularDrag; rb.inertiaTensor=body.inertiaTensor;
        rb.inertiaTensorRotation=body.inertiaTensorRotation; rb.useGravity=body.useGravity;
        wheels=new WheelCollider[4];
        bool planeCreated=false;
        for(int index=0;index<4;index++)
        {
            var source=original[index];
            var wheel=new GameObject("wheel-"+index); wheel.transform.parent=probe.transform;
            wheel.transform.position=source.transform.position+offset; wheel.transform.rotation=source.transform.rotation;
            var target=wheel.AddComponent<WheelCollider>(); wheels[index]=target;
            target.center=source.center; target.radius=source.radius; target.mass=source.mass;
            target.wheelDampingRate=source.wheelDampingRate; target.suspensionDistance=source.suspensionDistance;
            target.suspensionSpring=source.suspensionSpring; target.forceAppPointDistance=source.forceAppPointDistance;
            target.forwardFriction=source.forwardFriction; target.sidewaysFriction=source.sidewaysFriction;
            WheelHit hit;
            bool contact=source.GetGroundHit(out hit);
            if(!contact && sourceContactAvailable) { hit=sourceContact; contact=true; }
            Vector3 surfacePoint=hit.point, surfaceNormal=hit.normal;
            Collider surfaceCollider=hit.collider;
            string measurement="WheelCollider.GetGroundHit";
            if(!contact)
            {
                var origin=source.transform.TransformPoint(source.center)+source.transform.up*source.radius;
                var hits=Physics.RaycastAll(origin,-source.transform.up,10f,Physics.DefaultRaycastLayers,QueryTriggerInteraction.Ignore);
                foreach(var candidate in hits.OrderBy(candidate=>candidate.distance))
                {
                    if(candidate.collider.attachedRigidbody!=null) continue;
                    surfacePoint=candidate.point; surfaceNormal=candidate.normal; surfaceCollider=candidate.collider;
                    contact=true; measurement="Physics.Raycast against original static source geometry"; break;
                }
            }
            if(!planeCreated && contact)
            {
                var plane=GameObject.CreatePrimitive(PrimitiveType.Cube);
                plane.name="G11MeasuredContactPlane";
                plane.transform.rotation=Quaternion.FromToRotation(Vector3.up,surfaceNormal);
                plane.transform.localScale=new Vector3(200f,1f,200f);
                plane.transform.position=surfacePoint+offset-surfaceNormal*.5f;
                plane.GetComponent<Collider>().sharedMaterial=surfaceCollider.sharedMaterial;
                result.setup_changes.Add("Isolated 200x1x200 metre plane from source contact: "+
                    PathOf(surfaceCollider.transform)+" point="+surfacePoint+" normal="+surfaceNormal+" measured by "+measurement);
                planeCreated=true;
            }
        }
        if(!planeCreated) throw new InvalidOperationException("No measured source contact for isolated probe surface");
        G11Geometry.IsolateSourceCar();
        result.setup_changes.Add("Isolated probe keeps the original source car held with all native wheels disabled; assembled restoration is separate");
        body=rb; result.isolated_wheel_probe=true;
    }
    static Control Current()
    {
        int frame=active && startFrame>=0?Time.frameCount-startFrame:0;
        var selected=scenario.controls[0];
        for(int index=1;index<scenario.controls.Length;index++)
            if(scenario.controls[index].frame<=frame) selected=scenario.controls[index];
        return selected;
    }
    public static float Axis(inputscript input,inputscript.IN action)
    { return active?G11Controls.Axis(action,Time.frameCount-startFrame):0f; }
    public static bool Held(inputscript input,inputscript.IN action)
    { return active && G11Controls.Held(action,Time.frameCount-startFrame); }
    public static bool Down(inputscript input,inputscript.IN action)
    { return active && G11Controls.Down(action,Time.frameCount-startFrame); }
    public static bool Up(inputscript input,inputscript.IN action)
    { return active && G11Controls.Up(action,Time.frameCount-startFrame); }
    public static float AnalogAxis(string name) { return 0f; }
    void FixedUpdate()
    {
        if(finished) return;
        observedPhysicsStep++;
        try
        {
            G11BodyTrace.BeforeSolve();
            if(!active) return;
            if(startFrame<0) startFrame=Time.frameCount;
            if(G11AngularProbe.Active && step>=scenario.angular.duration_steps || G11ContactProbe.Active && step>=scenario.contact.duration_steps) return;
            step++;
            if(G11ContactProbe.Active) G11ContactProbe.Assign(step,observedPhysicsStep,Time.frameCount-startFrame,Time.time,Time.fixedDeltaTime);
            else if(G11AngularProbe.Active) G11AngularProbe.Assign(step);
            else if(result.isolated_wheel_probe)
            {
                var control=Current();
                for(int index=0;index<4;index++)
                {
                    wheels[index].motorTorque=control.motor_scale*car.brakePower;
                    wheels[index].brakeTorque=control.service_brake_fraction*car.brakePower+control.handbrake_fraction*car.maxHandBrake;
                    wheels[index].steerAngle=control.steer*car.steerAngle;
                }
            }
            Record("physics.entry");
        }
        catch(Exception error) { Fail(error); }
    }
    IEnumerator AfterPhysics()
    {
        var wait=new WaitForFixedUpdate();
        while(active && !finished)
        {
            yield return wait;
            try { if(step>result.physics_steps) { Record("physics.exit"); result.physics_steps=step; } }
            catch(Exception error) { Fail(error); }
        }
    }
    public static int InputRevision { get { return controlRevision; } }
    public static int ObservedPhysicsStep { get { return observedPhysicsStep; } }
    public static Rigidbody ObservedBody { get { return body; } }
    public static bool TraceOpen { get { return !finished; } }
    public static bool IsArmed { get { return active; } }
    public static void TraceFailure(Exception error) { Fail(error); }
    public static void SetSourceActive(GameObject target,bool value,string caller)
    {
        if(finished) target.SetActive(value);
        else G11Geometry.SetActive(target,value,caller);
    }
    public static void SetColliderEnabled(Collider collider,bool value)
    {
        if(finished) collider.enabled=value;
        else G11Geometry.SetEnabled(collider,value);
    }
    public static void ObserveLock(object owner,object cargo,string phase)
    {
        try { if(!finished) G11Geometry.LockBoundary((mountStuff)owner,(pickupable)cargo,phase,observedPhysicsStep,completedStorage); }
        catch(Exception error) { Fail(error); }
    }
    public static void CaptureDriverRejection(string callback)
    {
        G11Geometry.Capture("driver-readiness-rejected",observedPhysicsStep,callback,completedStorage);
    }
    static void RecordSetupBoundary(string phase)
    {
        G11BodyTrace.SetupBoundary(phase);
        var geometry=G11Geometry.Capture(phase,observedPhysicsStep,"observer.LateUpdate",completedStorage);
        if(!geometry.complete) throw new InvalidOperationException("Incomplete discovered physical geometry: "+String.Join("; ",geometry.unavailable));
        result.setup_boundaries.Add(new SetupBoundary{phase=phase,frame=Time.frameCount,time=Time.time,
            fixed_time=Time.fixedTime,delta_time=Time.deltaTime,kinematic=body.isKinematic,
            rotation=body.rotation,inertia_tensor=body.inertiaTensor,inertia_tensor_rotation=body.inertiaTensorRotation});
    }
    void LateUpdate()
    {
        try { if(!finished) G11Lifecycle.PollAudio("observer.LateUpdate"); }
        catch(Exception error) { Fail(error);return; }
        if(pendingDriver && !finished)
        {
            try
            {
                if(scenario.has_setup_state && scenario.setup_state.driver_input==null)
                    throw new InvalidOperationException("Canonical source driver input is missing");
                if(G11Driver.Tick(scenario.has_setup_state?scenario.setup_state.driver_input:null,observedPhysicsStep)) PrepareHeld();
            }
            catch(Exception error) { Fail(error); }
        }
        if(pendingArm && !finished && G11Driver.NextUpdateCompleted)
        {
            pendingArm=false;
            try
            {
                G11Driver.RequireHeld();
                RecordSetupBoundary("backlog-drained-held");
                if(preparedGeometry!=G11Geometry.Inputs(G11Geometry.snapshots[G11Geometry.snapshots.Count-1]))
                    throw new InvalidOperationException("Physical shape inputs changed across prepared-held and backlog-drained-held");
                result.held_conditions=ReadConditions(car);
                result.held_assembly=DescribeAssembly(car);
                if(!SourceAssemblyReady(car)) throw new InvalidOperationException("Source mounted assembly changed readiness during held boundary");
                RequireAssembly(result.prepared_assembly,result.held_assembly,"stable authored membership through held boundary");
                RecordSetupBoundary("before-dynamic");
                G11BodyTrace.ReleaseMeasurement(body);
                RecordSetupBoundary("switched-dynamic");
                if(scenario.setup.height=="wheels-clear") body.constraints=RigidbodyConstraints.FreezeAll;
                ApplyCanonical(result.setup_state);
                RecordSetupBoundary("canonical-applied");
                Physics.SyncTransforms();
                RecordSetupBoundary("canonical-transforms-synchronized");
                if(result.isolated_wheel_probe) foreach(var wheel in wheels) wheel.enabled=!G11ContactProbe.Active || G11ContactProbe.WheelEnabled;
                else G11Geometry.RestoreWheels();
                RecordSetupBoundary("native-wheels-enabled");
                if(G11AngularProbe.Active) {
                    Physics.SyncTransforms();
                    G11AngularProbe.Arm();
                    RecordSetupBoundary("angular-input-applied");
                }
                foreach(var collider in UnityEngine.Object.FindObjectsOfType<Collider>())
                    surfacePaths[collider.GetInstanceID()]=PathOf(collider.transform);
                if(G11ContactProbe.Active) {Physics.SyncTransforms();G11ContactProbe.Arm();RecordSetupBoundary("contact-input-applied");G11ContactProbe.RetainInitial();}
                if(G11AngularProbe.Active) G11AngularProbe.RetainInitial();
                result.initial=InitialState();
                RecordSetupBoundary("armed-after-canonical");
                G11Driver.Arm();
                G11Controls.Initialize(scenario.controls);
                startFrame=-1;
                active=true;
                StartCoroutine(AfterPhysics());
            }
            catch(Exception error) { Fail(error); }
        }
        if(active && !finished && G11ContactProbe.Active && startFrame>=0 && Time.frameCount-startFrame>=scenario.duration_frames && step<scenario.contact.duration_steps)
        { Fail(new InvalidOperationException("Contact render bound exceeded before required solves"));return; }
        if(active && !finished && G11AngularProbe.Active && startFrame>=0 && Time.frameCount-startFrame>=scenario.duration_frames && step<scenario.angular.duration_steps)
        { Fail(new InvalidOperationException("Angular declared render bound exceeded before required solves"));return; }
        if(active && !finished && startFrame>=0 && (G11ContactProbe.Active?step>=scenario.contact.duration_steps:G11AngularProbe.Active?step>=scenario.angular.duration_steps:Time.frameCount-startFrame>=scenario.duration_frames))
        {
            try { G11Geometry.NativeOwnershipCheckpoint("measurement-complete",ObservedPhysicsStep);G11BodyTrace.End(); result.completed=String.IsNullOrEmpty(result.error); Flush(); Application.Quit(result.completed?0:2); }
            catch(Exception error) { Fail(error); }
        }
    }
    static void Record(string phase)
    {
        lock(bufferGate) RecordLocked(phase);
    }
    static void RecordLocked(string phase)
    {
        if(finished) return;
        if(used>=buffer.Length) throw new InvalidOperationException("Native sample buffer exhausted");
        var sample=buffer[used];
        sample.phase=phase; sample.sequence=++sequence; sample.step=step; sample.frame=Time.frameCount-startFrame;
        sample.player_stored_dt=G11Driver.StoredDt;sample.player_consumed_dt=G11Driver.ConsumedDt;sample.player_update_sequence=G11Driver.UpdateSequence;
        sample.time=Time.time; sample.realtime=Time.realtimeSinceStartup; sample.delta_time=Time.deltaTime;
        sample.fixed_dt=Time.fixedDeltaTime; sample.input_sequence=lastInput; sample.car_sequence=lastCar;
        sample.engine_sequence=lastEngine; sample.rpm_sequence=lastRpm;
        sample.position=body.position; sample.rotation=body.rotation; sample.velocity=body.velocity;
        sample.angular_velocity=body.angularVelocity; sample.rpm=car.Engine.rpm; sample.average_rpm=car.avgRpm;
        sample.gear=car.gear; sample.clutch=car.clutch; sample.throttle=car.throttle;
        sample.brake=car.brake; sample.handbrake=car.handbrake;
        sample.controls=G11Controls.Read(car,driverSeat,controlRevision);
        sample.engine_output_nm=car.Engine.actualOutput; sample.engine_brake_nm=car.Engine.brakeOutput;
        sample.installed_driven_rims=0;
        foreach(var slot in car.DrivenWheelSlots) if(slot.part!=null) sample.installed_driven_rims++;
        sample.torque_divisor=car.DrivenWheelsNum();
        for(int index=0;index<wheels.Length;index++) sample.wheels[index]=ReadWheel(wheels[index]);
        if(G11AngularProbe.Active) G11AngularProbe.Read(sample.angular_state);
        if(G11ContactProbe.Active) G11ContactProbe.Read(sample.contact_state);
        for(int index=0;index<G11Geometry.nativeWheels.Length;index++)
            sample.native_wheels[index]=G11Geometry.ReadWheel(G11Geometry.nativeWheels[index]);
        buffer[used++]=sample;
    }
    static Wheel ReadWheel(WheelCollider collider)
    {
        var wheel=new Wheel(); WheelHit hit;
        wheel.contact=collider.GetGroundHit(out hit); wheel.rpm=collider.rpm;
        wheel.motor_torque=collider.motorTorque; wheel.brake_torque=collider.brakeTorque;
        wheel.steer_angle=collider.steerAngle; wheel.radius=collider.radius;
        wheel.suspension_distance=collider.suspensionDistance;
        collider.GetWorldPose(out wheel.pose,out wheel.rotation);
        wheel.suspension_offset_m=Vector3.Dot(collider.transform.TransformPoint(collider.center)-wheel.pose,collider.transform.up);
        if(wheel.contact)
        {
            wheel.contact_point=hit.point; wheel.contact_normal=hit.normal; wheel.force_n=hit.force;
            wheel.forward_slip=hit.forwardSlip; wheel.sideways_slip=hit.sidewaysSlip;
            wheel.forward=hit.forwardDir; wheel.sideways=hit.sidewaysDir;
            string path;
            wheel.surface=surfacePaths.TryGetValue(hit.collider.GetInstanceID(),out path)?path:"unavailable collider path";
        }
        return wheel;
    }
    static void Flush()
    {
        lock(bufferGate) FlushLocked();
    }
    static void FlushLocked()
    {
        if(finished) return; finished=true; active=false;
        if(timeoutTimer!=null) timeoutTimer.Dispose();
        Application.logMessageReceived-=CaptureLog;
        result.wheel_activation_events=new G11Geometry.Activation[G11Geometry.activation_count];
        Array.Copy(G11Geometry.activation_events,result.wheel_activation_events,G11Geometry.activation_count);
        result.samples=new Sample[used];
        if(buffer!=null) Array.Copy(buffer,result.samples,used);
        File.WriteAllText(Path.Combine(Output,"runtime.json"),JsonUtility.ToJson(result,true));
        File.WriteAllText(Path.Combine(Output,"completion.json"),"{\"completed\":"+
            (result.completed?"true":"false")+",\"sample_count\":"+used+",\"physics_steps\":"+result.physics_steps+"}");
        Debug.Log("G11_NATIVE_CAPTURE_FLUSHED completed="+result.completed+" samples="+used+" physics_steps="+result.physics_steps);
    }
    static void Fail(Exception error)
    {
        result.completed=false; result.error=error.ToString();
        Debug.LogException(error);
        try { Flush(); } catch(Exception flushError) { Debug.LogException(flushError); }
        Application.Quit(2);
    }
    void OnApplicationQuit() { Application.logMessageReceived-=CaptureLog; if(timeoutTimer!=null) timeoutTimer.Dispose(); if(!finished) { result.error="Player exited before finite capture completion"; Flush(); } }
}
