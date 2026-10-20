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
        public int frame, gear;
        public float throttle, brake, clutch, steer, handbrake, motor_scale;
        public bool ignition;
    }
    [Serializable] public class Scenario
    {
        public string id, probe_kind;
        public int duration_frames;
        public Setup setup;
        public Initial initial_state;
        public bool replay_initial;
        public Control[] controls;
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
        public float time, realtime, delta_time, fixed_dt, rpm, average_rpm, clutch, throttle, brake, handbrake;
        public float engine_output_nm, engine_brake_nm;
        public int gear, installed_driven_rims, torque_divisor;
        public Vector3 position, velocity, angular_velocity;
        public Quaternion rotation;
        public Wheel[] wheels;
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
    [Serializable] public class PoseInput { public Vector3 position; public Quaternion rotation; }
    [Serializable] public class Condition { public string path; public int state; }
    [Serializable] public class Initial
    {
        public string car_path, engine_path, driver_seat_path, scene, random_state, profile_path, setup_json;
        public int car_type;
        public float mass, fixed_dt, engine_rpm, engine_temperature;
        public Vector3 center_of_mass, position, velocity, angular_velocity, gravity;
        public Quaternion rotation;
        public string fuel_json, oil_json, coolant_json;
        public bool auto_clutch, auto_shift, ignition, running, stored_handbrake;
        public int selected_gear;
        public Wheel[] wheel_state;
        public Condition[] conditions;
        public PoseInput input_pose;
        public WheelParameters[] wheels;
    }
    [Serializable] public class Conditioning { public int step; public Initial before, after; }
    [Serializable] public class Result
    {
        public int schema_version=1;
        public string runtime="original-unity-player", scenario, probe_kind, error;
        public bool completed, menu_ready, lada_ready, control_capture_active, isolated_wheel_probe;
        public int physics_steps;
        public string units="metres; seconds; kilograms; degrees; rpm; newtons; newton metres; Unity WheelHit dimensionless slip";
        public Initial initial;
        public Sample[] samples;
        public List<Curve> curves=new List<Curve>();
        public List<Conditioning> conditioning=new List<Conditioning>();
        public List<string> setup_changes=new List<string>();
        public string[] unavailable={"WheelCollider internal angular inertia", "PhysX internal contact solver impulses"};
    }
    static G11Capture instance;
    static Scenario scenario;
    static readonly Result result=new Result();
    static carscript car;
    static seatscript driverSeat;
    static Rigidbody body;
    static WheelCollider[] wheels, sourceWheels;
    static PoseInput inputPose;
    static Sample[] buffer;
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
    public static void Observe(object owner,string phase)
    {
        try
        {
            if(phase=="car.ready")
            {
                var ready=(carscript)owner;
                if(ready.carType==itemdatabase.CarType.car02 && ready.RB!=null)
                {
                    ready.RB.isKinematic=true;
                    result.setup_changes.Add("Hold selected source Lada after original car Start, before native wheel history advances");
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
                result.scenario=scenario.id; result.probe_kind=scenario.probe_kind;
                new GameObject("G11NativeObserver").AddComponent<G11Capture>();
                instance.StartCoroutine(instance.Load((mainmenuscript)owner));
                return;
            }
            if(phase=="world.exit")
            {
                instance.StartCoroutine(instance.Initialize((mainscript)owner));
                return;
            }
            if(!active || finished) return;
            if(owner is carscript && owner!=car) return;
            if(owner is enginescript && owner!=car.Engine) return;
            if(phase=="input.entry")
            {
                if(lastInput==0) startFrame=Time.frameCount;
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
        // Wait for original Start callbacks and source part assembly, without draining or replacing them.
        for(int frame=0;frame<600;frame++)
        {
            yield return null;
            car=UnityEngine.Object.FindObjectsOfType<carscript>().SingleOrDefault(c=>c.carType==itemdatabase.CarType.car02);
            if(car!=null && car.RB!=null && car.Engine!=null && car.driverSeat!=null && car.allWH.Count==4 && main.player!=null)
                break;
        }
        try
        {
            if(car==null || car.Engine==null || car.RB==null || car.allWH.Count!=4)
                throw new InvalidOperationException("Original selected Lada did not initialize");
            body=car.RB; sourceWheels=car.allWH.ToArray(); wheels=sourceWheels;
            ApplySetup(main);
            foreach(var wheel in wheels) if(wheel.GetGroundHit(out sourceContact)) { sourceContactAvailable=true; break; }
            body.isKinematic=true;
            foreach(var wheel in wheels) wheel.enabled=false;
            result.setup_changes.Add("Hold body and disable native wheels during setup; enable native wheels at observation start");
            yieldPending=true;
        }
        catch(Exception error) { Fail(error); }
        if(!yieldPending) yield break;
        // Allow original refresh coroutines and WheelCollider property updates to consume setup changes.
        for(int frame=0;frame<120;frame++) yield return null;
        try
        {
            if(!main.player.mind) throw new InvalidOperationException("Original source player initialization is incomplete");
            main.PressedContinue();
            driverSeat=car.GetComponentsInChildren<seatscript>().Single(seat=>seat.driverSeat0 && seat.Car==car);
            main.player.GetIn(driverSeat);
            if(!car.driving || !car.isPlayerDriving || main.player.Car!=car)
                throw new InvalidOperationException("Original selected Lada controls did not activate");
            result.lada_ready=true;
            result.setup_changes.Add("Original mainscript.PressedContinue and fpscontroller.GetIn selected Lada after initialization");
            if(scenario.probe_kind=="wheel_collider") CreateIsolatedProbe();
            foreach(var wheel in wheels) wheel.enabled=false;
            if(scenario.replay_initial) ReplayInitial(scenario.initial_state);
            else
            {
                body.mass*=scenario.setup.load_multiplier;
                if(scenario.setup.surface=="nearby-source-slope") PlaceOnSourceSlope();
                if(scenario.setup.height=="one-source-suspension-drop") body.position+=Vector3.up*wheels[0].suspensionDistance;
                if(scenario.setup.height=="wheels-clear") body.position+=Vector3.up*(wheels[0].radius*4+wheels[0].suspensionDistance);
            }
            inputPose=scenario.replay_initial?scenario.initial_state.input_pose:
                new PoseInput{position=body.transform.position,rotation=body.transform.rotation};
            body.position=inputPose.position; body.rotation=inputPose.rotation;
            Physics.SyncTransforms();
            body.isKinematic=false;
            foreach(var wheel in wheels) wheel.enabled=true;
            result.setup_changes.Add("Apply the same pre-normalization pose input once; synchronize native transforms; enable fresh native wheel state");
            body.velocity=body.transform.TransformDirection(scenario.setup.velocity_mps);
            body.angularVelocity=Vector3.zero;
            if(scenario.setup.height=="wheels-clear")
            {
                body.constraints=RigidbodyConstraints.FreezeAll;
                result.setup_changes.Add("Hold lifted source body with rigid constraints; native wheels remain free");
            }
            foreach(var collider in UnityEngine.Object.FindObjectsOfType<Collider>())
                surfacePaths[collider.GetInstanceID()]=PathOf(collider.transform);
            if(!Physics.autoSimulation) throw new InvalidOperationException("Original native auto simulation is inactive");
            result.setup_changes.Add("Two unpowered original native physics conditioning steps with the same reset input between them");
        }
        catch(Exception error) { Fail(error); }
        if(finished) yield break;
        float beforeCondition=Time.fixedTime;
        result.conditioning.Add(new Conditioning{step=1,before=InitialState()});
        yield return new WaitForFixedUpdate();
        try
        {
            result.conditioning[0].after=InitialState();
            foreach(var wheel in wheels) wheel.enabled=false;
            body.position=inputPose.position; body.rotation=inputPose.rotation;
            body.velocity=body.transform.TransformDirection(scenario.setup.velocity_mps);
            body.angularVelocity=Vector3.zero;
            Physics.SyncTransforms();
            foreach(var wheel in wheels) wheel.enabled=true;
            result.conditioning.Add(new Conditioning{step=2,before=InitialState()});
        }
        catch(Exception error) { Fail(error); }
        if(finished) yield break;
        beforeCondition=Time.fixedTime;
        yield return new WaitForFixedUpdate();
        try
        {
            if(!Physics.autoSimulation || Time.fixedTime<=beforeCondition)
                throw new InvalidOperationException("Native conditioning physics step did not advance");
            body.velocity=body.transform.TransformDirection(scenario.setup.velocity_mps);
            body.angularVelocity=Vector3.zero;
            result.initial=InitialState();
            result.conditioning[1].after=result.initial;
            CaptureCurves();
            // Reserve all callback samples and wheel records before the finite observation interval.
            buffer=new Sample[(scenario.duration_frames+1)*256];
            for(int index=0;index<buffer.Length;index++) buffer[index].wheels=new Wheel[4];
            startFrame=Time.frameCount;
            active=true;
            StartCoroutine(AfterPhysics());
        }
        catch(Exception error) { Fail(error); }
    }
    bool yieldPending;
    static void ApplySetup(mainscript main)
    {
        main.crsrLocked=true;
        settingsscript.s.S.BAutoClutch=scenario.setup.auto_clutch;
        settingsscript.s.S.BAutoShift=scenario.setup.auto_shift;
        settingsscript.s.S.BForceManual=false;
        settingsscript.s.S.CBenableGas=false; settingsscript.s.S.CBenableBrake=false;
        settingsscript.s.S.CBenableClutch=false; settingsscript.s.S.CBenableSteer=false;
        settingsscript.s.S.CBenableHandBrake=false;
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
    static void ReplayInitial(Initial initial)
    {
        body.mass=initial.mass;
        body.centerOfMass=initial.center_of_mass;
        UnityEngine.Random.state=JsonUtility.FromJson<UnityEngine.Random.State>(initial.random_state);
        car.Engine.rpm=initial.engine_rpm; car.Engine.temp=initial.engine_temperature;
        car.ignition=initial.ignition; car.Engine.running=initial.running; car.BhandBrake=initial.stored_handbrake;
        car.ShiftTo(initial.selected_gear,true);
        if(car.Tank!=null) JsonUtility.FromJsonOverwrite(initial.fuel_json,car.Tank.F);
        if(car.Engine.T!=null) JsonUtility.FromJsonOverwrite(initial.oil_json,car.Engine.T.F);
        if(car.coolant!=null) JsonUtility.FromJsonOverwrite(initial.coolant_json,car.coolant.coolant.F);
        result.setup_changes.Add("Replay first repeat's native initial body, engine, gear, fluid, and handbrake values");
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
            setup_json=JsonUtility.ToJson(scenario.setup), input_pose=inputPose, mass=body.mass, center_of_mass=body.centerOfMass,
            position=body.position, rotation=body.rotation, velocity=body.velocity, angular_velocity=body.angularVelocity,
            fixed_dt=Time.fixedDeltaTime, gravity=Physics.gravity, engine_rpm=car.Engine.rpm, engine_temperature=car.Engine.temp,
            auto_clutch=settingsscript.s.S.BAutoClutch,auto_shift=settingsscript.s.S.BAutoShift,
            ignition=car.ignition,running=car.Engine.running,stored_handbrake=car.BhandBrake,selected_gear=car.gear,
            wheel_state=wheels.Select(w=>ReadWheel(w)).ToArray(),
            conditions=car.GetComponentsInChildren<partconditionscript>().Select(c=>new Condition{path=PathOf(c.transform),state=c.state}).ToArray(),
            fuel_json=car.Tank==null?"unavailable":JsonUtility.ToJson(car.Tank.F),
            oil_json=car.Engine.T==null?"unavailable":JsonUtility.ToJson(car.Engine.T.F),
            coolant_json=car.coolant==null?"unavailable":JsonUtility.ToJson(car.coolant.coolant.F),
            wheels=wheels.Select((w,index)=>new WheelParameters {path=PathOf(w.transform),local_position=w.transform.localPosition,
                local_rotation=w.transform.localRotation,center=w.center,radius=w.radius,mass=w.mass,damping=w.wheelDampingRate,
                suspension_distance=w.suspensionDistance,force_application_distance=w.forceAppPointDistance,
                spring_n_per_m=w.suspensionSpring.spring,damper_n_s_per_m=w.suspensionSpring.damper,
                spring_target=w.suspensionSpring.targetPosition,forward=new Friction(w.forwardFriction),sideways=new Friction(w.sidewaysFriction),
                driven=car.WHDriven.Contains(sourceWheels[index]),steer=car.WHSteer.Contains(sourceWheels[index]),service_brake=car.WHBrakeOnly.Contains(sourceWheels[index]),
                handbrake=car.WHHandBrake.Contains(sourceWheels[index])}).ToArray() };
    }
    static void CaptureCurves()
    {
        var fields=typeof(carscript).GetFields().Where(f=>f.FieldType==typeof(AnimationCurve));
        foreach(var field in fields) CaptureCurve("carscript",field.Name,(AnimationCurve)field.GetValue(car));
        CaptureCurve("enginescript","torqueCurve",car.Engine.torqueCurve);
    }
    static void CaptureCurve(string owner,string name,AnimationCurve curve)
    {
        if(curve==null || curve.length==0) return;
        var keys=curve.keys;
        // The outside values use one source key interval, rather than a recreated extrapolation formula.
        float interval=keys.Length>1?keys[keys.Length-1].time-keys[0].time:1f;
        var inputs=new List<float>{0f,keys[0].time-interval};
        foreach(var key in keys) inputs.Add(key.time);
        inputs.Add(keys[keys.Length-1].time+interval);
        result.curves.Add(new Curve{owner=owner,name=name,keys=keys.Select(k=>new Key(k)).ToArray(),pre_wrap=(int)curve.preWrapMode,
            post_wrap=(int)curve.postWrapMode,inputs=inputs.ToArray(),values=inputs.Select(curve.Evaluate).ToArray()});
    }
    static void CreateIsolatedProbe()
    {
        // Copy live source WheelCollider properties. No tire or engine formula runs in this probe.
        var original=wheels;
        var offset=Vector3.right*10000f;
        var probe=new GameObject("G11IsolatedNativeWheelProbe");
        probe.transform.position=body.position+offset; probe.transform.rotation=body.rotation;
        var rb=probe.AddComponent<Rigidbody>(); rb.mass=body.mass; rb.centerOfMass=body.centerOfMass;
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
        body=rb; result.isolated_wheel_probe=true;
    }
    static Control Current()
    {
        int frame=active?Time.frameCount-startFrame:0;
        var selected=scenario.controls[0];
        for(int index=1;index<scenario.controls.Length;index++)
            if(scenario.controls[index].frame<=frame) selected=scenario.controls[index];
        return selected;
    }
    public static float Axis(inputscript input,inputscript.IN action)
    {
        if(!active) return 0f;
        var control=Current();
        switch(action)
        {
            case inputscript.IN.gas:return control.throttle;
            case inputscript.IN.brake:return control.brake;
            case inputscript.IN.clutch:return control.clutch;
            case inputscript.IN.steerleft:return Mathf.Max(0f,-control.steer);
            case inputscript.IN.steerright:return Mathf.Max(0f,control.steer);
            default:return 0f;
        }
    }
    public static bool Held(inputscript input,inputscript.IN action)
    {
        if(!active) return false;
        var control=Current();
        if(action==inputscript.IN.ignition) return control.ignition;
        return Axis(input,action)>0f;
    }
    public static bool Down(inputscript input,inputscript.IN action)
    {
        if(!active) return false;
        var control=Current();
        if(Time.frameCount-startFrame!=control.frame) return false;
        if(action==inputscript.IN.handbrake) return control.handbrake>0;
        if(action==inputscript.IN.ignition) return control.ignition;
        if(action==inputscript.IN.gearr) return control.gear==-1;
        if(action==inputscript.IN.gearn) return control.gear==0;
        if(action>=inputscript.IN.gear1 && action<=inputscript.IN.gear6)
            return control.gear==(int)action-(int)inputscript.IN.gear1+1;
        return false;
    }
    public static bool Up(inputscript input,inputscript.IN action)
    {
        if(!active) return false;
        int frame=Time.frameCount-startFrame;
        for(int index=1;index<scenario.controls.Length;index++)
            if(scenario.controls[index].frame==frame && action==inputscript.IN.ignition)
                return scenario.controls[index-1].ignition && !scenario.controls[index].ignition;
        return false;
    }
    public static float AnalogAxis(string name) { return 0f; }
    void FixedUpdate()
    {
        if(!active || finished) return;
        try
        {
            step++;
            if(result.isolated_wheel_probe)
            {
                var control=Current();
                for(int index=0;index<4;index++)
                {
                    wheels[index].motorTorque=control.motor_scale*car.brakePower;
                    wheels[index].brakeTorque=control.brake*car.brakePower+control.handbrake*car.maxHandBrake;
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
    void LateUpdate()
    {
        if(active && !finished && Time.frameCount-startFrame>=scenario.duration_frames)
        {
            try { result.completed=String.IsNullOrEmpty(result.error); Flush(); Application.Quit(result.completed?0:2); }
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
        sample.time=Time.time; sample.realtime=Time.realtimeSinceStartup; sample.delta_time=Time.deltaTime;
        sample.fixed_dt=Time.fixedDeltaTime; sample.input_sequence=lastInput; sample.car_sequence=lastCar;
        sample.engine_sequence=lastEngine; sample.rpm_sequence=lastRpm;
        sample.position=body.position; sample.rotation=body.rotation; sample.velocity=body.velocity;
        sample.angular_velocity=body.angularVelocity; sample.rpm=car.Engine.rpm; sample.average_rpm=car.avgRpm;
        sample.gear=car.gear; sample.clutch=car.clutch; sample.throttle=car.throttle;
        sample.brake=car.brake; sample.handbrake=car.handbrake;
        sample.engine_output_nm=car.Engine.actualOutput; sample.engine_brake_nm=car.Engine.brakeOutput;
        sample.installed_driven_rims=0;
        foreach(var slot in car.DrivenWheelSlots) if(slot.part!=null) sample.installed_driven_rims++;
        sample.torque_divisor=car.DrivenWheelsNum();
        for(int index=0;index<4;index++) sample.wheels[index]=ReadWheel(wheels[index]);
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
