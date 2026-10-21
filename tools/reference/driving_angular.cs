using System;
using System.Collections.Generic;
using UnityEngine;

// Native measurements only. Rig inputs do not define source physics or wheel inertia.
public static class G11AngularProbe
{
    [System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Explicit,Size=4)]
    public struct FloatWord {
        [System.Runtime.InteropServices.FieldOffset(0)] public float value;
        [System.Runtime.InteropServices.FieldOffset(0)] public int word;
    }
    public static int Word(float value) { var view=new FloatWord{value=value};return view.word; }
    [Serializable] public struct VectorWords {
        public int x,y,z;
        public VectorWords(Vector3 value) { x=Word(value.x);y=Word(value.y);z=Word(value.z); }
    }
    [Serializable] public struct QuaternionWords {
        public int x,y,z,w;
        public QuaternionWords(Quaternion value) { x=Word(value.x);y=Word(value.y);z=Word(value.z);w=Word(value.w); }
    }
    [Serializable] public struct BodyWords {
        public int mass;public VectorWords center_of_mass,inertia_tensor;public QuaternionWords inertia_tensor_rotation;
        public BodyWords(float mass,Vector3 center,Vector3 tensor,Quaternion rotation) {
            this.mass=Word(mass);center_of_mass=new VectorWords(center);inertia_tensor=new VectorWords(tensor);inertia_tensor_rotation=new QuaternionWords(rotation);
        }
    }
    [Serializable] public struct ProfileWords {
        public int mass,radius,damping;
        public ProfileWords(Profile value) { mass=Word(value.mass);radius=Word(value.radius);damping=Word(value.damping); }
    }
    [Serializable] public struct ParsedWords {
        public bool observed,canonical_supplied,canonical_is_null;
        public VectorWords setup_velocity,rig_velocity,rig_angular_velocity;
        public BodyWords canonical_arguments;
    }
    [Serializable] public struct InitialWords {
        public bool observed;public int wheel_id,fixed_dt,rpm;
        public VectorWords velocity,angular_velocity;
    }
    [Serializable] public struct SampleWords {
        public bool observed;public int wheel_id,sequence,step,fixed_dt,motor_nm,brake_nm,rpm;public string phase;
    }
    static ParsedWords parsedWords;
    public static void ObserveAfterFromJson(Vector3 setupVelocity,Input declared,bool supplied,BodyInput canonical) {
        parsedWords=new ParsedWords{observed=true,canonical_supplied=supplied,canonical_is_null=canonical==null,
            setup_velocity=new VectorWords(setupVelocity),rig_velocity=new VectorWords(declared.rig.velocity_mps),
            rig_angular_velocity=new VectorWords(declared.rig.angular_velocity_rad_s)};
        if(supplied && canonical!=null) parsedWords.canonical_arguments=new BodyWords(canonical.mass,canonical.center_of_mass,canonical.inertia_tensor,canonical.inertia_tensor_rotation);
    }
    public static bool IsSelectedWheel(WheelCollider candidate) { return System.Object.ReferenceEquals(candidate,wheel); }
    public static void ObserveInitial(WheelCollider selected,float fixedDt,float rpm) {
        if(!IsSelectedWheel(selected) || report.initial_state==null) return;
        var initial=report.initial_state;
        report.initial_words=new InitialWords{observed=true,wheel_id=initial.wheel_id,fixed_dt=Word(fixedDt),rpm=Word(rpm),
            velocity=new VectorWords(initial.body.velocity),angular_velocity=new VectorWords(initial.body.angular_velocity)};
    }
    public static SampleWords ObserveSample(WheelCollider selected,State state,int sequence,int step,string phase,float fixedDt,float motor,float brake,float rpm) {
        if(!IsSelectedWheel(selected)) return new SampleWords();
        return new SampleWords{observed=true,wheel_id=state.wheel_id,sequence=sequence,step=step,phase=phase,
            fixed_dt=Word(fixedDt),motor_nm=Word(motor),brake_nm=Word(brake),rpm=Word(rpm)};
    }

    [Serializable] public class Event { public int step; public string phase; public float motor_nm,brake_nm; }
    [Serializable] public class Rig {
        public bool use_gravity; public float drag,angular_drag,sleep_threshold; public int constraints;
        public Vector3 velocity_mps,angular_velocity_rad_s,offset_m;
    }
    [Serializable] public class Input {
        public string support,purpose,source_wheel,provenance,body_properties,schedule_unit;
        public int duration_steps; public Event[] events; public Rig rig;
    }
    [Serializable] public struct Friction {
        public float extremum_slip,extremum_value,asymptote_slip,asymptote_value,stiffness;
        public Friction(WheelFrictionCurve curve) { extremum_slip=curve.extremumSlip;extremum_value=curve.extremumValue;
            asymptote_slip=curve.asymptoteSlip;asymptote_value=curve.asymptoteValue;stiffness=curve.stiffness; }
    }
    [Serializable] public class Profile {
        public Vector3 center;
        public float radius,mass,damping,suspension_distance,force_application_distance,spring_n_per_m,damper_n_s_per_m,spring_target;
        public Friction forward,sideways;
    }
    [Serializable] public struct Body {
        public int instance_id,constraints,solver_iterations,solver_velocity_iterations;
        public float mass,drag,angular_drag,sleep_threshold,max_angular_velocity;
        public bool sleeping,use_gravity,detect_collisions,kinematic;
        public int interpolation,collision_detection_mode;
        public Vector3 center_of_mass,inertia_tensor,position,velocity,angular_velocity,gravity;
        public Quaternion rotation,inertia_tensor_rotation;
    }
    [Serializable] public class State : Profile {
        public int event_revision,assignment_step,wheel_id;
        public string phase;
        public bool enabled,active_self,active_hierarchy,sprung_mass_available;
        public float sprung_mass; public Body body;
    }
    [Serializable] public class Assignment { public int event_revision,step; public string phase;public float motor_nm,brake_nm; }
    [Serializable] public class BodyInput {
        public float mass;public Vector3 center_of_mass,inertia_tensor;public Quaternion inertia_tensor_rotation;
    }
    [Serializable] public class TransformInput {
        public Vector3 position,local_scale;public Quaternion rotation;
    }
    [Serializable] public class Report {
        public int word_schema_version=1;public ParsedWords parsed_words;public BodyWords argument_words,source_argument_words;
        public ProfileWords source_profile_words,copied_profile_words;public InitialWords initial_words;
        public Input declared;public string source_key,support;
        public int source_wheel_id,copied_wheel_id,original_body_id,copied_body_id;
        public Profile source_profile,copied_profile;public Body source_body,copied_constructor_body,copied_held_body;
        public float source_sprung_mass,copied_sprung_mass;
        public TransformState source_transform,copied_transform;
        public BodyInput body_input;public bool body_input_from_source;public TransformInput wheel_input;
        public Assignment[] assignments;
        public State initial_state;
    }
    [Serializable] public struct TransformState {
        public Vector3 position,local_position,local_scale,lossy_scale;public Quaternion rotation,local_rotation;
        public TransformState(Transform value) { position=value.position;rotation=value.rotation;local_position=value.localPosition;
            local_rotation=value.localRotation;local_scale=value.localScale;lossy_scale=value.lossyScale; }
    }
    public static Report report;
    static Input input;
    static WheelCollider wheel;
    static Rigidbody body;
    static int revision,assignedStep;
    static bool armed;
    public static bool Active { get { return input!=null; } }
    public static void ReadProfile(WheelCollider value,Profile target) {
        target.center=value.center;target.radius=value.radius;target.mass=value.mass;target.damping=value.wheelDampingRate;
        target.suspension_distance=value.suspensionDistance;target.force_application_distance=value.forceAppPointDistance;
        target.spring_n_per_m=value.suspensionSpring.spring;target.damper_n_s_per_m=value.suspensionSpring.damper;
        target.spring_target=value.suspensionSpring.targetPosition;target.forward=new Friction(value.forwardFriction);target.sideways=new Friction(value.sidewaysFriction);
    }
    public static Body ReadBody(Rigidbody value) {
        return new Body{instance_id=value.GetInstanceID(),constraints=(int)value.constraints,mass=value.mass,center_of_mass=value.centerOfMass,
            inertia_tensor=value.inertiaTensor,inertia_tensor_rotation=value.inertiaTensorRotation,position=value.position,rotation=value.rotation,
            velocity=value.velocity,angular_velocity=value.angularVelocity,gravity=Physics.gravity,use_gravity=value.useGravity,
            drag=value.drag,angular_drag=value.angularDrag,sleeping=value.IsSleeping(),sleep_threshold=value.sleepThreshold,
            solver_iterations=value.solverIterations,solver_velocity_iterations=value.solverVelocityIterations,
            max_angular_velocity=value.maxAngularVelocity,detect_collisions=value.detectCollisions,kinematic=value.isKinematic,
            interpolation=(int)value.interpolation,collision_detection_mode=(int)value.collisionDetectionMode};
    }
    public static WheelCollider Configure(Input declared,WheelCollider source,Rigidbody original,GameObject probe,Rigidbody held,bool hasCanonical,BodyInput canonical) {
        if(input!=null || !held.isKinematic || !original.isKinematic || G11Geometry.Key(source)!=declared.source_wheel)
            throw new InvalidOperationException("Angular source/copy hold or identity differs");
        if(hasCanonical != (canonical!=null))
            throw new InvalidOperationException("Angular canonical body input is missing or unexpected");
        input=declared;body=held;var constructorBody=ReadBody(body);var sourceBody=ReadBody(original);
        var bodyInput=hasCanonical?canonical:new BodyInput{mass=sourceBody.mass,center_of_mass=sourceBody.center_of_mass,
            inertia_tensor=sourceBody.inertia_tensor,inertia_tensor_rotation=sourceBody.inertia_tensor_rotation};
        var argumentWords=new BodyWords(bodyInput.mass,bodyInput.center_of_mass,bodyInput.inertia_tensor,bodyInput.inertia_tensor_rotation);
        body.mass=bodyInput.mass;body.centerOfMass=bodyInput.center_of_mass;
        body.inertiaTensor=bodyInput.inertia_tensor;body.inertiaTensorRotation=bodyInput.inertia_tensor_rotation;
        body.constraints=(RigidbodyConstraints)input.rig.constraints;body.useGravity=input.rig.use_gravity;
        body.drag=input.rig.drag;body.angularDrag=input.rig.angular_drag;body.sleepThreshold=input.rig.sleep_threshold;
        var child=new GameObject("one-native-wheel");child.transform.parent=probe.transform;
        var wheelInput=new TransformInput{position=source.transform.position+input.rig.offset_m,
            rotation=source.transform.rotation,local_scale=source.transform.lossyScale};
        child.transform.position=wheelInput.position;child.transform.rotation=wheelInput.rotation;
        child.transform.localScale=wheelInput.local_scale;
        wheel=child.AddComponent<WheelCollider>();wheel.enabled=false;
        wheel.center=source.center;wheel.radius=source.radius;wheel.mass=source.mass;wheel.wheelDampingRate=source.wheelDampingRate;
        wheel.suspensionDistance=source.suspensionDistance;wheel.suspensionSpring=source.suspensionSpring;
        wheel.forceAppPointDistance=source.forceAppPointDistance;wheel.forwardFriction=source.forwardFriction;wheel.sidewaysFriction=source.sidewaysFriction;
        wheel.motorTorque=0;wheel.brakeTorque=0;wheel.steerAngle=0;
        report=new Report{parsed_words=parsedWords,argument_words=argumentWords,
            source_argument_words=new BodyWords(sourceBody.mass,sourceBody.center_of_mass,sourceBody.inertia_tensor,sourceBody.inertia_tensor_rotation),
            declared=input,source_key=G11Geometry.Key(source),support=input.support,
            source_wheel_id=source.GetInstanceID(),copied_wheel_id=wheel.GetInstanceID(),original_body_id=original.GetInstanceID(),copied_body_id=body.GetInstanceID(),
            source_profile=new Profile(),copied_profile=new Profile(),source_body=sourceBody,copied_constructor_body=constructorBody,copied_held_body=ReadBody(body),
            source_sprung_mass=source.sprungMass,copied_sprung_mass=wheel.sprungMass,
            source_transform=new TransformState(source.transform),copied_transform=new TransformState(wheel.transform),body_input=bodyInput,body_input_from_source=!hasCanonical,wheel_input=wheelInput,assignments=new Assignment[input.events.Length]};
        ReadProfile(source,report.source_profile);ReadProfile(wheel,report.copied_profile);
        report.source_profile_words=new ProfileWords(report.source_profile);report.copied_profile_words=new ProfileWords(report.copied_profile);
        for(int i=0;i<report.assignments.Length;i++) report.assignments[i]=new Assignment();
        return wheel;
    }
    public static void Arm() {
        if(!Active) return;
        if(armed || body.isKinematic || !wheel.enabled || !wheel.gameObject.activeSelf || !wheel.gameObject.activeInHierarchy)
            throw new InvalidOperationException("Angular arming repeated or copied body/wheel is not active");
        // One declared input after copied-wheel activation. Never restore state during measurement.
        body.velocity=input.rig.velocity_mps;body.angularVelocity=input.rig.angular_velocity_rad_s;
        armed=true;
    }
    public static void RetainInitial() { report.initial_state=new State();Read(report.initial_state); }
    public static void Assign(int step) {
        if(!Active || step!=assignedStep+1 || step>input.duration_steps)
            throw new InvalidOperationException("Angular consuming solve repeated or missing");
        if(revision<input.events.Length && input.events[revision].step==step) {
            var next=input.events[revision];
            if(next.phase.StartsWith("coast",StringComparison.Ordinal) && (next.motor_nm!=0 || next.brake_nm!=0))
                throw new InvalidOperationException("Angular coast input is not zero");
            wheel.motorTorque=next.motor_nm;wheel.brakeTorque=next.brake_nm;wheel.steerAngle=0;
            var assignment=report.assignments[revision];assignment.event_revision=revision+1;assignment.step=step;assignment.phase=next.phase;
            assignment.motor_nm=wheel.motorTorque;assignment.brake_nm=wheel.brakeTorque;
            revision++;
        }
        if(revision==0) throw new InvalidOperationException("Angular initial assignment missing");
        assignedStep=step;
    }
    public static void Read(State target) {
        if(!Active) return;
        ReadProfile(wheel,target);target.body=ReadBody(body);target.wheel_id=wheel.GetInstanceID();
        target.enabled=wheel.enabled;target.active_self=wheel.gameObject.activeSelf;target.active_hierarchy=wheel.gameObject.activeInHierarchy;
        target.event_revision=revision;target.assignment_step=assignedStep;target.phase=revision==0?"before-first-assignment":input.events[revision-1].phase;
        try { target.sprung_mass=wheel.sprungMass;target.sprung_mass_available=true; }
        catch { target.sprung_mass_available=false; }
    }
}
