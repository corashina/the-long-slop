using System;
using System.Linq;
using UnityEngine;

// Copied-rig inputs and boundary observations. No native force law is inferred.
public static class G11ContactProbe
{
    [Serializable] public class Input {
        public string source_wheel,support,mode,schedule_unit,provenance;
        public int duration_steps; public bool wheel_enabled; public float side_speed_mps,torque_scale;
        public int[] event_steps;public Vector3 offset_m;
    }
    [Serializable] public struct Observation {
        public G11AngularProbe.Body body,original_body;
        public Vector3 body_scale,body_lossy_scale,world_center_of_mass,wheel_center,wheel_up,wheel_position,wheel_scale,wheel_lossy_scale;
        public Quaternion wheel_rotation,pose_rotation;public Vector3 pose_position;
        public int wheel_id,owner_id;public bool owner_is_null,owner_available,enabled,active_self,active_hierarchy;
        public bool body_parent_is_null,wheel_parent_is_body;
        public bool rpm_available,sprung_mass_available,contact,hit_available,body_point_velocity_available,attached_body_available,attached_point_velocity_available;
        public float rpm,sprung_mass,q,motor_nm,brake_nm,steer_degrees;
        public int collider_id,attached_body_id;public Vector3 point,normal,forward_direction,sideways_direction,body_point_velocity,attached_point_velocity;
        public float forward_slip,sideways_slip,force_scalar;
    }
    [Serializable] public struct PreInput {
        public int step,observed_step,frame,input_revision; public float time,fixed_dt;
        public Observation observed;public int native_wheel_count;
        public G11Geometry.NativeWheel native_wheel0,native_wheel1,native_wheel2,native_wheel3,native_wheel4;
    }
    [Serializable] public class State : G11AngularProbe.Profile {
        public int step,event_revision,assignment_step; public PreInput pre_input; public Observation observed;
    }
    [Serializable] public class Assignment {
        public int step,event_revision;public float motor_argument_nm,brake_argument_nm,steer_argument_degrees,motor_nm,brake_nm,steer_degrees;
    }
    [Serializable] public class Impulse {
        public int producer_revision,consuming_step,application_count,mode;
        public float mass,side_speed_mps;public Vector3 pre_velocity,target_velocity,argument,application_point;
    }
    [Serializable] public class Geometry {
        public Vector3 query_origin,query_direction,source_point,normal,forward,sideways,source_forward,source_up;
        public Vector3 source_center,source_world_com,source_center_offset,copied_center,copied_com_offset,copied_held_world_com;
        public float query_distance,source_signed_height; public int query_mask,source_collider_id,source_material_id,support_id,support_material_id;
        public string selection_rule,source_collider_key;public Vector3 support_position,support_scale,position_spacing;
        public Quaternion support_rotation;
    }
    [Serializable] public class Report {
        public Input declared;public int other_external_input_count;public string source_key;public int source_wheel_id,copied_wheel_id,original_body_id,copied_body_id,source_index,road_role;
        public G11AngularProbe.Profile source_profile,copied_profile;
        public G11AngularProbe.Body source_body,copied_constructor_body,copied_held_body;
        public G11AngularProbe.TransformState source_transform,copied_transform,source_body_transform,copied_body_transform;
        public G11AngularProbe.BodyInput body_input;public bool body_input_from_source;
        public G11AngularProbe.TransformInput wheel_input;public Geometry geometry;
        public bool source_sprung_mass_available,copied_sprung_mass_available;public float source_sprung_mass,copied_sprung_mass;
        public float n0,q0,motor_argument;public Assignment[] assignments;public Impulse impulse;public State initial_state;
    }
    [Serializable] public struct SetupMark {
        public int frame,physics_step,input_revision;
    }
    [Serializable] public struct ParsedReadout {
        public bool observed,from_source,canonical_is_null;public SetupMark mark;
    }
    [Serializable] public struct ConfigureReadout {
        public bool observed,active,copied_kinematic,source_kinematic,from_source,canonical_is_null;
        public int copied_body_id,source_body_id,source_wheel_id;public string source_component_key,declared_source_key;
        public bool reject_active,reject_copied_dynamic,reject_source_dynamic,reject_source_key,reject_canonical_origin;
        public SetupMark mark;
    }
    [Serializable] public class Readout {
        public ParsedReadout after_from_json;public ConfigureReadout before_guard;
    }
    public static readonly Readout readout=new Readout();
    public static void ObserveAfterFromJson(bool fromSource,G11AngularProbe.BodyInput canonical,SetupMark mark)
    {
        readout.after_from_json=new ParsedReadout{observed=true,from_source=fromSource,canonical_is_null=canonical==null,mark=mark};
    }
    public static Report report;
    static Input input;static WheelCollider wheel;static Rigidbody body,original;static Collider support;
    static PreInput[] preInputs;static int assignedStep,revision;static bool armed;
    public static bool Active { get {return input!=null;} }
    public static bool WheelEnabled { get {return input.wheel_enabled;} }
    static float Spacing(float x) {if(x==0) return float.Epsilon;return (float)Math.Pow(2,Math.Floor(Math.Log(Math.Abs(x),2))-23);}
    public static WheelCollider Configure(Input declared,WheelCollider source,Rigidbody sourceBody,GameObject probe,Rigidbody held,bool fromSource,G11AngularProbe.BodyInput canonical,SetupMark mark)
    {
        var observed=new ConfigureReadout{observed=true,active=Active,copied_body_id=held.GetInstanceID(),copied_kinematic=held.isKinematic,
            source_body_id=sourceBody.GetInstanceID(),source_kinematic=sourceBody.isKinematic,source_wheel_id=source.GetInstanceID(),
            source_component_key=G11Geometry.Key(source),declared_source_key=declared.source_wheel,from_source=fromSource,canonical_is_null=canonical==null,mark=mark};
        observed.reject_active=observed.active;observed.reject_copied_dynamic=!observed.copied_kinematic;
        observed.reject_source_dynamic=!observed.source_kinematic;observed.reject_source_key=observed.source_component_key!=observed.declared_source_key;
        observed.reject_canonical_origin=observed.from_source!=observed.canonical_is_null;
        readout.before_guard=observed;
        if(observed.reject_active || observed.reject_copied_dynamic || observed.reject_source_dynamic || observed.reject_source_key || observed.reject_canonical_origin)
            throw new InvalidOperationException("Contact source, hold, or canonical origin differs");
        if(declared.duration_steps!=240 || declared.mode!="force" || declared.support!="full-tire" || declared.event_steps==null || declared.event_steps.Length!=3 || declared.event_steps[0]!=1 || declared.event_steps[1]!=121 || declared.event_steps[2]!=145)
            throw new InvalidOperationException("Contact fixed schedule differs");
        input=declared;body=held;original=sourceBody;
        var constructor=G11AngularProbe.ReadBody(body);var raw=G11AngularProbe.ReadBody(original);
        var center=source.transform.TransformPoint(source.center);var origin=center+source.transform.up*source.radius;
        var hits=Physics.RaycastAll(origin,-source.transform.up,10f,Physics.DefaultRaycastLayers,QueryTriggerInteraction.Ignore);
        bool found=false;RaycastHit chosen=new RaycastHit();float nearest=float.PositiveInfinity;
        foreach(var hit in hits) if(!hit.collider.isTrigger && hit.collider.attachedRigidbody==null && !hit.collider.transform.IsChildOf(original.transform) && hit.distance<nearest) {found=true;chosen=hit;nearest=hit.distance;}
        if(!found) throw new InvalidOperationException("Contact measured static support missing");
        var n=chosen.normal.normalized;var projected=source.transform.forward-n*Vector3.Dot(source.transform.forward,n);
        if(projected.sqrMagnitude==0 || Vector3.Dot(Physics.gravity,n)>=0) throw new InvalidOperationException("Contact basis or opposing gravity missing");
        var f=projected.normalized;var s=Vector3.Cross(n,f);var rotation=Quaternion.LookRotation(f,n);
        var copiedCenter=chosen.point+input.offset_m+n*(source.radius+source.suspensionDistance*.5f);
        probe.transform.localScale=Vector3.one;probe.transform.position=copiedCenter;probe.transform.rotation=rotation;
        var sourceOffset=original.worldCenterOfMass-center;var height=Vector3.Dot(sourceOffset,source.transform.up);
        var comOffset=n*height;
        var bodyInput=fromSource?new G11AngularProbe.BodyInput {mass=raw.mass/4f,center_of_mass=new Vector3(0f,height,0f),inertia_tensor=raw.inertia_tensor,inertia_tensor_rotation=raw.inertia_tensor_rotation}:canonical;
        // Sole owner of copied body properties. Retain setter quaternion separately from native normalization.
        body.mass=bodyInput.mass;body.centerOfMass=bodyInput.center_of_mass;body.inertiaTensor=bodyInput.inertia_tensor;body.inertiaTensorRotation=bodyInput.inertia_tensor_rotation;
        body.useGravity=true;body.drag=0;body.angularDrag=0;body.constraints=RigidbodyConstraints.FreezeRotation;
        var plane=GameObject.CreatePrimitive(PrimitiveType.Cube);plane.name="G11ContactMeasuredSupport";
        plane.transform.rotation=Quaternion.FromToRotation(Vector3.up,n);plane.transform.localScale=new Vector3(200f,1f,200f);plane.transform.position=chosen.point+input.offset_m-n*.5f;
        support=plane.GetComponent<Collider>();support.sharedMaterial=chosen.collider.sharedMaterial;
        var child=new GameObject("one-contact-wheel");child.transform.parent=probe.transform;child.transform.localScale=Vector3.one;child.transform.rotation=rotation;
        child.transform.position=copiedCenter-child.transform.TransformVector(source.center);
        wheel=child.AddComponent<WheelCollider>();wheel.enabled=false;
        wheel.center=source.center;wheel.radius=source.radius;wheel.mass=source.mass;wheel.wheelDampingRate=source.wheelDampingRate;
        wheel.suspensionDistance=source.suspensionDistance;wheel.suspensionSpring=source.suspensionSpring;wheel.forceAppPointDistance=source.forceAppPointDistance;wheel.forwardFriction=source.forwardFriction;wheel.sidewaysFriction=source.sidewaysFriction;
        wheel.motorTorque=0;wheel.brakeTorque=0;wheel.steerAngle=0;
        float n0=bodyInput.mass*Math.Abs(Vector3.Dot(Physics.gravity,n));float q0=n0*source.radius;
        report=new Report {declared=input,source_key=G11Geometry.Key(source),source_index=Array.IndexOf(G11Geometry.nativeWheels,source),road_role=G11Geometry.ReadWheel(source).road_slot,source_wheel_id=source.GetInstanceID(),copied_wheel_id=wheel.GetInstanceID(),original_body_id=original.GetInstanceID(),copied_body_id=body.GetInstanceID(),
            source_body=raw,copied_constructor_body=constructor,copied_held_body=G11AngularProbe.ReadBody(body),body_input=bodyInput,body_input_from_source=fromSource,
            source_profile=new G11AngularProbe.Profile(),copied_profile=new G11AngularProbe.Profile(),source_body_transform=new G11AngularProbe.TransformState(original.transform),copied_body_transform=new G11AngularProbe.TransformState(probe.transform),source_transform=new G11AngularProbe.TransformState(source.transform),copied_transform=new G11AngularProbe.TransformState(wheel.transform),
            wheel_input=new G11AngularProbe.TransformInput {position=child.transform.position,rotation=rotation,local_scale=Vector3.one},n0=n0,q0=q0,motor_argument=input.torque_scale*q0,
            geometry=new Geometry {query_origin=origin,query_direction=-source.transform.up,query_distance=10f,query_mask=Physics.DefaultRaycastLayers,selection_rule="nearest non-trigger static hit outside original car hierarchy",source_collider_key=G11Geometry.Key(chosen.collider),source_collider_id=chosen.collider.GetInstanceID(),source_material_id=chosen.collider.sharedMaterial==null?0:chosen.collider.sharedMaterial.GetInstanceID(),source_point=chosen.point,normal=n,forward=f,sideways=s,source_forward=source.transform.forward,source_up=source.transform.up,source_center=center,source_world_com=original.worldCenterOfMass,source_center_offset=sourceOffset,source_signed_height=height,copied_center=copiedCenter,copied_com_offset=body.worldCenterOfMass-copiedCenter,copied_held_world_com=body.worldCenterOfMass,
                support_id=support.GetInstanceID(),support_material_id=support.sharedMaterial==null?0:support.sharedMaterial.GetInstanceID(),support_position=plane.transform.position,support_scale=plane.transform.localScale,support_rotation=plane.transform.rotation,position_spacing=new Vector3(Spacing(copiedCenter.x),Spacing(copiedCenter.y),Spacing(copiedCenter.z))},
            assignments=new Assignment[3],impulse=new Impulse(),initial_state=new State()};
        for(int i=0;i<3;i++)report.assignments[i]=new Assignment();
        G11AngularProbe.ReadProfile(source,report.source_profile);G11AngularProbe.ReadProfile(wheel,report.copied_profile);
        try {report.source_sprung_mass=source.sprungMass;report.source_sprung_mass_available=true;}catch {report.source_sprung_mass_available=false;}
        try {report.copied_sprung_mass=wheel.sprungMass;report.copied_sprung_mass_available=true;}catch {report.copied_sprung_mass_available=false;}
        preInputs=new PreInput[240];return wheel;
    }
    public static void Arm() {
        if(!Active || armed || body.isKinematic || wheel.enabled!=input.wheel_enabled || !wheel.gameObject.activeSelf || !wheel.gameObject.activeInHierarchy) throw new InvalidOperationException("Contact arming state differs");
        body.velocity=5f*report.geometry.forward;body.angularVelocity=Vector3.zero;body.WakeUp();armed=true;
    }
    public static void RetainInitial() {Read(report.initial_state);}
    public static void Assign(int step,int observedStep,int frame,float time,float dt) {
        if(!armed || step!=assignedStep+1 || step>preInputs.Length) throw new InvalidOperationException("Contact consuming solve repeated, missing, or exhausted");
        if(G11Geometry.nativeWheels.Length!=5)throw new InvalidOperationException("Contact original native cohort count differs");
        var before=Observe();preInputs[step-1]=new PreInput {native_wheel_count=5,native_wheel0=G11Geometry.ReadWheel(G11Geometry.nativeWheels[0]),native_wheel1=G11Geometry.ReadWheel(G11Geometry.nativeWheels[1]),native_wheel2=G11Geometry.ReadWheel(G11Geometry.nativeWheels[2]),native_wheel3=G11Geometry.ReadWheel(G11Geometry.nativeWheels[3]),native_wheel4=G11Geometry.ReadWheel(G11Geometry.nativeWheels[4]),step=step,observed_step=observedStep,frame=frame,time=time,fixed_dt=dt,input_revision=revision,observed=before};
        if(revision<3 && input.event_steps[revision]==step) {
            if(revision==1) {
                if(report.impulse.application_count!=0) throw new InvalidOperationException("Contact impulse repeated");
                var v=before.body.velocity;var target=5f*report.geometry.forward+input.side_speed_mps*report.geometry.sideways+Vector3.Dot(v,report.geometry.normal)*report.geometry.normal;
                var argument=report.body_input.mass*(target-v);
                var impulse=report.impulse;impulse.mass=report.body_input.mass;impulse.pre_velocity=v;impulse.target_velocity=target;impulse.side_speed_mps=input.side_speed_mps;
                impulse.argument=argument;impulse.application_point=before.world_center_of_mass;impulse.mode=(int)ForceMode.Impulse;impulse.producer_revision=revision+1;impulse.consuming_step=step;
                body.AddForce(argument,ForceMode.Impulse);impulse.application_count++;
            }
            float motorArgument=revision==1?report.motor_argument:0f,brakeArgument=0f,steerArgument=0f;
            wheel.motorTorque=motorArgument;wheel.brakeTorque=brakeArgument;wheel.steerAngle=steerArgument;
            var a=report.assignments[revision];a.step=step;a.event_revision=revision+1;
            a.motor_argument_nm=motorArgument;a.brake_argument_nm=brakeArgument;a.steer_argument_degrees=steerArgument;
            a.motor_nm=wheel.motorTorque;a.brake_nm=wheel.brakeTorque;a.steer_degrees=wheel.steerAngle;revision++;
        }
        if(revision==0 || revision<3 && input.event_steps[revision]<step) throw new InvalidOperationException("Contact assignment missing or late");
        assignedStep=step;
        if(step==240 && (revision!=3 || report.impulse.application_count!=1)) throw new InvalidOperationException("Contact event completion differs");
    }
    static Observation Observe() {
        var value=new Observation {body_parent_is_null=body.transform.parent==null,wheel_parent_is_body=wheel.transform.parent==body.transform,body=G11AngularProbe.ReadBody(body),original_body=G11AngularProbe.ReadBody(original),body_scale=body.transform.localScale,body_lossy_scale=body.transform.lossyScale,world_center_of_mass=body.worldCenterOfMass,
            wheel_id=wheel.GetInstanceID(),enabled=wheel.enabled,active_self=wheel.gameObject.activeSelf,active_hierarchy=wheel.gameObject.activeInHierarchy,
            wheel_center=wheel.transform.TransformPoint(wheel.center),wheel_up=wheel.transform.up,wheel_position=wheel.transform.position,wheel_rotation=wheel.transform.rotation,wheel_scale=wheel.transform.localScale,wheel_lossy_scale=wheel.transform.lossyScale,
            motor_nm=wheel.motorTorque,brake_nm=wheel.brakeTorque,steer_degrees=wheel.steerAngle};
        var owner=wheel.attachedRigidbody;value.owner_is_null=owner==null;value.owner_id=owner==null?0:owner.GetInstanceID();value.owner_available=owner!=null;
        try {value.rpm=wheel.rpm;value.rpm_available=true;}catch {value.rpm_available=false;}
        try {value.sprung_mass=wheel.sprungMass;value.sprung_mass_available=true;}catch {value.sprung_mass_available=false;}
        wheel.GetWorldPose(out value.pose_position,out value.pose_rotation);value.q=Vector3.Dot(value.wheel_center-value.pose_position,value.wheel_up);
        WheelHit hit;value.contact=wheel.GetGroundHit(out hit);value.hit_available=value.contact;
        if(value.contact) {
            value.collider_id=hit.collider==null?0:hit.collider.GetInstanceID();value.point=hit.point;value.normal=hit.normal;value.forward_direction=hit.forwardDir;value.sideways_direction=hit.sidewaysDir;value.forward_slip=hit.forwardSlip;value.sideways_slip=hit.sidewaysSlip;value.force_scalar=hit.force;
            value.body_point_velocity=body.GetPointVelocity(hit.point);value.body_point_velocity_available=true;
            var attached=hit.collider==null?null:hit.collider.attachedRigidbody;value.attached_body_available=attached!=null;value.attached_body_id=attached==null?0:attached.GetInstanceID();
            if(attached!=null){value.attached_point_velocity=attached.GetPointVelocity(hit.point);value.attached_point_velocity_available=true;}
        }
        return value;
    }
    public static void Read(State target) {
        G11AngularProbe.ReadProfile(wheel,target);target.step=assignedStep;target.event_revision=revision;target.assignment_step=assignedStep;
        target.observed=Observe();if(assignedStep>0)target.pre_input=preInputs[assignedStep-1];
    }
}
