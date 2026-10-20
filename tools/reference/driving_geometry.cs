using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEngine;

// Setup-only snapshots. Sample wheel storage is allocated before observation starts.
public static class G11Geometry
{
    [Serializable] public struct Activation
    {
        public string wheel,caller;public int frame;public bool previous,requested,observed;
    }
    [Serializable] public class SourceComponent { public string type,key,material; }
    [Serializable] public class SourceTransform { public string path,key; public SourceComponent[] components; }
    [Serializable] public class SourceMap { public string root; public SourceTransform[] transforms; }
    [Serializable] public struct NativeWheel
    {
        public bool enabled,active_self,active_hierarchy,contact,sprung_mass_available;
        public float motor_torque,brake_torque,rpm,sprung_mass,hit_force;
        public Vector3 hit_point,hit_normal;
        public string contact_response;
    }
    [Serializable] public class Material
    {
        public string key; public float dynamic_friction,static_friction,bounciness;
        public int friction_combine,bounce_combine;
    }
    [Serializable] public class Shape
    {
        public string key,type,transform,owner,source_owner,unknown_shape;
        public int instance_id,owner_instance_id,layer,road_slot;
        public bool enabled,active_self,active_hierarchy,is_trigger,complete,owner_is_null,material_available;
        public bool saved_enabled,held_enabled,intended_enabled,restored_enabled,restoration_applied;
        public float contact_offset,radius,height,mass,damping,suspension_distance,force_application_distance,sprung_mass;
        public bool sprung_mass_available;
        public int direction;
        public Vector3 center,size;
        public float spring_n_per_m,damper_n_s_per_m,spring_target;
        public G11Capture.Friction forward,sideways;
        public Material material;
        public NativeWheel sample;
        public Matrix4x4 world_matrix,body_matrix;
    }
    [Serializable] public class Pose
    {
        public string key,parent; public int instance_id;
        public Vector3 local_position,local_scale,lossy_scale;
        public Quaternion local_rotation;
        public bool active_self,active_hierarchy;
        public Matrix4x4 world_matrix,body_matrix;
    }
    [Serializable] public class Body
    {
        public string key,parent,transform_key; public int instance_id,constraints,interpolation;
        public Vector3 position,transform_position,center_of_mass,inertia_tensor,local_scale,lossy_scale;
        public Quaternion rotation,transform_rotation,inertia_tensor_rotation;
        public float mass; public bool kinematic,detect_collisions;
        public Matrix4x4 world_matrix;
    }
    [Serializable] public class Cargo
    {
        public string key,parent,lock_owner,original_identity; public int category,item,id_in_save,lock_parent_id,lock_id;
        public bool locked,body_present,original_body_present,deferred_destruction,ownership_ready,retained_nested_body;
        public float body_mass;
        public Vector3 world_position,local_position,local_scale;
        public Quaternion world_rotation,local_rotation;
        public string[] collider_owners;
    }
    [Serializable] public class Mount
    {
        public string key,parent,source_parent; public int id; public bool complete,no_physics_lock;
        public string[] stored;
    }
    [Serializable] public class Slot
    {
        public string key,part,condition; public int source_index; public bool crafted;
    }
    [Serializable] public class Engine
    {
        public Pose tmove,engine_move; public Pose[] trot;
        public float rpm,max_vibrate,max_rotate,max_vibrate_speed; public bool ignition;
    }
    [Serializable] public class Snapshot
    {
        public string phase,source_callback,random_state;
        public int frame,physics_step; public float time,fixed_time,delta_time;
        public bool complete,physical_ready;
        public Body body; public Body[] nested_bodies; public Shape[] colliders; public Pose[] transforms;
        public Mount[] mounts; public Cargo[] cargo; public Slot[] slots; public Engine engine;
        public string[] unavailable;
        public int observer_body_instance_id; public bool observer_body_present,observer_body_is_current;
        public string readiness_failure;
    }
    class Hold { public bool saved,intended,restored,applied; }
    class Locked
    {
        public mountStuff mount; public pickupable cargo; public Rigidbody original_body;
        public Collider[] colliders; public Rigidbody[] original_owners; public string original;
    }
    static carscript car;
    static readonly Dictionary<Transform,string> keys=new Dictionary<Transform,string>();
    static readonly Dictionary<Component,string> componentKeys=new Dictionary<Component,string>();
    static readonly Dictionary<PhysicMaterial,string> materialKeys=new Dictionary<PhysicMaterial,string>();
    static readonly Dictionary<WheelCollider,Hold> held=new Dictionary<WheelCollider,Hold>();
    static readonly List<Locked> locked=new List<Locked>();
    public static readonly List<Snapshot> snapshots=new List<Snapshot>();
    public static WheelCollider[] nativeWheels;
    static bool holding;
    public static Activation[] activation_events=new Activation[4096];
    public static int activation_count;
    static string Segment(Transform value)
    {
        int occurrence=0;
        if(value.parent!=null) for(int index=0;index<value.GetSiblingIndex();index++)
            if(value.parent.GetChild(index).name==value.name) occurrence++;
        return value.name+"["+occurrence+"]";
    }
    static string Relative(Transform root,Transform value)
    {
        if(root==value) return ".";
        var parts=new List<string>();
        for(var current=value;current!=root;current=current.parent)
        {
            if(current==null) throw new InvalidOperationException("Geometry relative identity is outside its owner");
            parts.Add(Segment(current));
        }
        parts.Reverse();return String.Join("/",parts.ToArray());
    }
    public static string Key(Transform value)
    {
        if(value==null) return null;
        string key;
        if(keys.TryGetValue(value,out key)) return key;
        var part=value.GetComponentInParent<tosaveitemscript>();
        if(part!=null && part.transform!=value)
            key=Key(part.transform)+"/"+Relative(part.transform,value);
        else key=(value.parent==null?"scene:"+value.gameObject.scene.name:Key(value.parent))+"/"+Segment(value);
        keys.Add(value,key); return key;
    }
    public static string Key(Component value)
    {
        if(value==null) return null;
        string key;
        if(componentKeys.TryGetValue(value,out key)) return key;
        var components=value.GetComponents(value.GetType());int index=Array.IndexOf(components,value);
        key=Key(value.transform)+"/"+value.GetType().Name+"["+index+"]";
        componentKeys.Add(value,key);return key;
    }
    public static void Initialize(carscript target)
    {
        car=target;
        var map=JsonUtility.FromJson<SourceMap>(File.ReadAllText(Path.Combine(Environment.GetEnvironmentVariable("G11_RUN_ROOT"),"../../source-geometry-map.json")));
        var byPath=map.transforms.ToDictionary(row=>row.path);
        var seen=new HashSet<string>();
        foreach(var transform in car.GetComponentsInChildren<Transform>(true))
        {
            SourceTransform source;string path=Relative(car.transform,transform);
            if(!byPath.TryGetValue(path,out source) || !seen.Add(path)) throw new InvalidOperationException("Unmapped or ambiguous authored geometry: "+path);
            keys[transform]=source.key;
            var groups=new Dictionary<string,int>();
            foreach(var component in transform.GetComponents<Component>())
            {
                if(component==null) continue;
                string type=component.GetType().Name;
                if(!(component is Collider) && !(component is Rigidbody)) continue;
                int ordinal;groups.TryGetValue(type,out ordinal); groups[type]=ordinal+1;
                var matches=source.components.Where(row=>row.type==type).ToArray();
                if(ordinal>=matches.Length) throw new InvalidOperationException("Unmapped authored physical component: "+path+"/"+type);
                componentKeys[component]=matches[ordinal].key;
                var collider=component as Collider;
                if(collider!=null && collider.sharedMaterial!=null) materialKeys[collider.sharedMaterial]=matches[ordinal].material;
            }
        }
        if(seen.Count!=map.transforms.Length) throw new InvalidOperationException("Incomplete authored geometry hierarchy mapping");
        // Cache original scene identities before storage reparents cargo. No measured pose enters a key.
        foreach(var item in Resources.FindObjectsOfTypeAll<tosaveitemscript>())
            if(item.gameObject.scene.IsValid()) Key(item.transform);
        nativeWheels=car.GetComponentsInChildren<WheelCollider>(true);
        foreach(var wheel in nativeWheels) held.Add(wheel,new Hold{saved=wheel.enabled,intended=wheel.enabled});
    }
    public static void HoldWheels()
    {
        holding=true;
        foreach(var wheel in nativeWheels)
        {
            Hold state;
            if(!held.TryGetValue(wheel,out state)) { state=new Hold{saved=wheel.enabled,intended=wheel.enabled};held.Add(wheel,state); }
            wheel.enabled=false;
        }
    }
    public static void SetEnabled(Collider collider,bool value)
    {
        var wheel=collider as WheelCollider;Hold state;
        if(wheel!=null && holding && held.TryGetValue(wheel,out state)) { state.intended=value;wheel.enabled=false; }
        else collider.enabled=value;
    }
    public static void SetActive(GameObject target,bool value,string caller)
    {
        bool previous=target.activeSelf;target.SetActive(value);
        if(nativeWheels==null || !holding) return;
        foreach(var wheel in nativeWheels) if(wheel.gameObject==target)
        {
            if(activation_count>=activation_events.Length) throw new InvalidOperationException("Source wheel activation event buffer exhausted");
            activation_events[activation_count++]=new Activation{wheel=Key(wheel),caller=caller,frame=Time.frameCount,
                previous=previous,requested=value,observed=target.activeSelf};
        }
    }
    public static void RestoreWheels()
    {
        holding=false;
        foreach(var pair in held) { pair.Key.enabled=pair.Value.intended;pair.Value.restored=pair.Key.enabled;pair.Value.applied=true; }
    }
    public static void IsolateSourceCar() { holding=true;foreach(var wheel in nativeWheels) wheel.enabled=false; }
    public static void LockBoundary(mountStuff mount,pickupable cargo,string phase,int step,HashSet<mountStuff> completed)
    {
        if(car==null || !mount.transform.IsChildOf(car.transform)) return;
        if(phase=="storage.lock.entry")
        {
            var colliders=cargo.GetComponentsInChildren<Collider>(true);
            if(!locked.Any(value=>value.mount==mount && value.cargo==cargo))
            locked.Add(new Locked{mount=mount,cargo=cargo,original_body=cargo.RB!=null?cargo.RB:cargo.GetComponent<Rigidbody>(),
                colliders=colliders,original_owners=colliders.Select(c=>c.attachedRigidbody).ToArray(),original=Key(cargo.transform)});
        }
        Capture(phase,step,phase,completed,cargo);
    }
    static bool Ready(Locked item)
    {
        if(item.cargo==null || !item.mount.stored.Contains(item.cargo)) return true;
        if(item.cargo.ragdoll!=null) return true; // Retained nested ownership stays explicit in cargo and shape records.
        if(item.original_body!=null || item.cargo.GetComponent<Rigidbody>()!=null) return false;
        for(int index=0;index<item.colliders.Length;index++)
        {
            var shape=item.colliders[index];if(shape==null) return false;
            var expected=item.original_owners[index];
            // Destroyed original owner becomes Unity null. Retained nested bodies must not transfer.
            if(shape.attachedRigidbody!=(expected==null?car.RB:expected)) return false;
        }
        return true;
    }
    public static string ReadinessFailure()
    {
        foreach(var item in locked)
        {
            if(item.cargo==null || !item.mount.stored.Contains(item.cargo)) continue;
            if(item.cargo.ragdoll!=null) continue;
            if(item.original_body!=null || item.cargo.GetComponent<Rigidbody>()!=null)
                return "Deferred source Rigidbody still present: "+item.original;
            for(int index=0;index<item.colliders.Length;index++)
            {
                var shape=item.colliders[index];if(shape==null) return "Source cargo collider was destroyed: "+item.original;
                var expected=item.original_owners[index];
                if(shape.attachedRigidbody!=(expected==null?car.RB:expected))
                    return "Cargo owner transfer incomplete: "+Key(shape)+" observed="+Key(shape.attachedRigidbody)+" expected="+Key(expected==null?car.RB:expected)+
                        " enabled="+shape.enabled+" activeSelf="+shape.gameObject.activeSelf+" activeHierarchy="+shape.gameObject.activeInHierarchy;
            }
        }
        foreach(var mount in car.GetComponentsInChildren<mountStuff>(true)) foreach(var cargo in mount.stored)
            if(cargo!=null && cargo.ragdoll==null)
            {
                if(cargo.GetComponent<Rigidbody>()!=null) return "Stored cargo Rigidbody present: "+Key(cargo.transform);
                if(!locked.Any(item=>item.mount==mount && item.cargo==cargo)) return "Stored cargo has no observed Lock owner: "+Key(cargo.transform);
            }
        return null;
    }
    public static bool PhysicalReady()
    {
        foreach(var item in locked) if(!Ready(item)) return false;
        foreach(var mount in car.GetComponentsInChildren<mountStuff>(true)) foreach(var cargo in mount.stored)
            if(cargo!=null && cargo.ragdoll==null)
            {
                if(cargo.GetComponent<Rigidbody>()!=null || !locked.Any(item=>item.mount==mount && item.cargo==cargo)) return false;
                foreach(var shape in cargo.GetComponentsInChildren<Collider>(true))
                    if(shape.GetComponentInParent<Rigidbody>()==car.RB && shape.attachedRigidbody!=car.RB) return false;
            }
        return true;
    }
    static Pose ReadPose(Transform value)
    {
        if(value==null) return null;
        return new Pose{key=Key(value),parent=Key(value.parent),instance_id=value.GetInstanceID(),
            local_position=value.localPosition,local_rotation=value.localRotation,local_scale=value.localScale,
            lossy_scale=value.lossyScale,active_self=value.gameObject.activeSelf,active_hierarchy=value.gameObject.activeInHierarchy,
            world_matrix=value.localToWorldMatrix,body_matrix=car.RB.transform.worldToLocalMatrix*value.localToWorldMatrix};
    }
    static Body ReadBody(Rigidbody value)
    {
        return new Body{key=Key(value),transform_key=Key(value.transform),parent=Key(value.transform.parent),instance_id=value.GetInstanceID(),position=value.position,
            rotation=value.rotation,transform_position=value.transform.position,transform_rotation=value.transform.rotation,
            center_of_mass=value.centerOfMass,mass=value.mass,inertia_tensor=value.inertiaTensor,inertia_tensor_rotation=value.inertiaTensorRotation,
            kinematic=value.isKinematic,constraints=(int)value.constraints,detect_collisions=value.detectCollisions,interpolation=(int)value.interpolation,
            local_scale=value.transform.localScale,lossy_scale=value.transform.lossyScale,world_matrix=value.transform.localToWorldMatrix};
    }
    public static NativeWheel ReadWheel(WheelCollider wheel)
    {
        WheelHit hit;bool contact=wheel.GetGroundHit(out hit);
        return new NativeWheel{enabled=wheel.enabled,active_self=wheel.gameObject.activeSelf,active_hierarchy=wheel.gameObject.activeInHierarchy,
            motor_torque=wheel.motorTorque,brake_torque=wheel.brakeTorque,rpm=wheel.rpm,sprung_mass=wheel.sprungMass,sprung_mass_available=true,
            contact=contact,hit_point=contact?hit.point:Vector3.zero,hit_normal=contact?hit.normal:Vector3.zero,hit_force=contact?hit.force:0,
            contact_response=contact?"native WheelCollider.GetGroundHit response":"native GetGroundHit false; hit response unavailable"};
    }
    static Shape ReadShape(Collider value)
    {
        var owner=value.attachedRigidbody;
        var shape=new Shape{key=Key(value),instance_id=value.GetInstanceID(),type=value.GetType().Name,transform=Key(value.transform),
            owner=Key(owner),owner_is_null=owner==null,owner_instance_id=owner==null?0:owner.GetInstanceID(),source_owner=Key(value.GetComponentInParent<tosaveitemscript>()),
            enabled=value.enabled,active_self=value.gameObject.activeSelf,active_hierarchy=value.gameObject.activeInHierarchy,is_trigger=value.isTrigger,
            layer=value.gameObject.layer,contact_offset=value.contactOffset,road_slot=-1,complete=true,world_matrix=value.transform.localToWorldMatrix,
            body_matrix=car.RB.transform.worldToLocalMatrix*value.transform.localToWorldMatrix};
        var material=value.sharedMaterial;
        string materialKey=material==null?null:(materialKeys.ContainsKey(material)?materialKeys[material]:"runtime-material:"+material.name);
        shape.material_available=material!=null;
        if(material!=null) shape.material=new Material{key=materialKey,dynamic_friction=material.dynamicFriction,static_friction=material.staticFriction,
            bounciness=material.bounciness,friction_combine=(int)material.frictionCombine,bounce_combine=(int)material.bounceCombine};
        var box=value as BoxCollider;var sphere=value as SphereCollider;var capsule=value as CapsuleCollider;var wheel=value as WheelCollider;
        if(box!=null) { shape.center=box.center;shape.size=box.size; }
        else if(sphere!=null) { shape.center=sphere.center;shape.radius=sphere.radius; }
        else if(capsule!=null) { shape.center=capsule.center;shape.radius=capsule.radius;shape.height=capsule.height;shape.direction=capsule.direction; }
        else if(wheel!=null)
        {
            shape.center=wheel.center;shape.radius=wheel.radius;shape.mass=wheel.mass;shape.damping=wheel.wheelDampingRate;
            shape.suspension_distance=wheel.suspensionDistance;shape.force_application_distance=wheel.forceAppPointDistance;
            shape.spring_n_per_m=wheel.suspensionSpring.spring;shape.damper_n_s_per_m=wheel.suspensionSpring.damper;shape.spring_target=wheel.suspensionSpring.targetPosition;
            shape.forward=new G11Capture.Friction(wheel.forwardFriction);shape.sideways=new G11Capture.Friction(wheel.sidewaysFriction);
            shape.sprung_mass=wheel.sprungMass;shape.sprung_mass_available=true;shape.sample=ReadWheel(wheel);shape.road_slot=car.allWH.IndexOf(wheel);
            Hold state;if(held.TryGetValue(wheel,out state)) { shape.saved_enabled=state.saved;shape.held_enabled=false;shape.intended_enabled=state.intended;
                shape.restored_enabled=state.restored;shape.restoration_applied=state.applied; }
        }
        else { shape.complete=false;shape.unknown_shape="Native shape data unavailable for discovered "+shape.type; }
        return shape;
    }
    static Cargo ReadCargo(Locked item)
    {
        var cargo=item.cargo;var save=cargo.GetComponent<tosaveitemscript>();var rb=cargo.GetComponent<Rigidbody>();
        return new Cargo{key=Key(cargo.transform),original_identity=item.original,parent=Key(cargo.transform.parent),lock_owner=Key(item.mount),
            category=save==null?-1:save.category,item=save==null?-1:save.id,id_in_save=save==null?-1:save.idInSave,
            locked=cargo.isPhyslocked,lock_parent_id=cargo.physLockParentID,lock_id=cargo.physLockID,body_present=rb!=null,
            original_body_present=item.original_body!=null,deferred_destruction=item.mount.stored.Contains(cargo) && item.original_body!=null,
            ownership_ready=Ready(item),retained_nested_body=cargo.ragdoll!=null,body_mass=rb==null?0:rb.mass,
            world_position=cargo.transform.position,world_rotation=cargo.transform.rotation,local_position=cargo.transform.localPosition,
            local_rotation=cargo.transform.localRotation,local_scale=cargo.transform.localScale,
            collider_owners=item.colliders.Select(shape=>shape==null?"destroyed collider":Key(shape.attachedRigidbody)).ToArray()};
    }
    public static string Inputs(Snapshot snapshot)
    {
        var copy=JsonUtility.FromJson<Snapshot>(JsonUtility.ToJson(snapshot));
        foreach(var shape in copy.colliders)
        {
            shape.instance_id=shape.owner_instance_id=0;shape.world_matrix=shape.body_matrix=default(Matrix4x4);
            shape.sample=default(NativeWheel);shape.sprung_mass=0;
            shape.saved_enabled=shape.held_enabled=shape.restored_enabled=shape.restoration_applied=false;
        }
        foreach(var pose in copy.transforms)
        {
            pose.instance_id=0;pose.world_matrix=pose.body_matrix=default(Matrix4x4);pose.lossy_scale=Vector3.zero;
            if(pose.key==copy.body.transform_key) { pose.local_position=Vector3.zero;pose.local_rotation=default(Quaternion); }
        }
        return JsonUtility.ToJson(new InputTable{colliders=copy.colliders,transforms=copy.transforms});
    }
    [Serializable] class InputTable { public Shape[] colliders;public Pose[] transforms; }
    [Serializable] public class RestField { public string key,value; }
    public static RestField[] DriverRest(Collider[] colliders,Transform[] transforms)
    {
        var result=new List<RestField>();
        foreach(var collider in colliders)
        {
            if(collider==null) throw new InvalidOperationException("Retained driver collider destroyed during rest observation");
            var shape=ReadShape(collider);shape.instance_id=shape.owner_instance_id=0;shape.world_matrix=shape.body_matrix=default(Matrix4x4);
            result.Add(new RestField{key=shape.key,value=JsonUtility.ToJson(shape)});
        }
        foreach(var transform in transforms)
        {
            if(transform==null) throw new InvalidOperationException("Retained driver transform destroyed during rest observation");
            var pose=ReadPose(transform);pose.instance_id=0;pose.world_matrix=pose.body_matrix=default(Matrix4x4);pose.lossy_scale=Vector3.zero;
            result.Add(new RestField{key=pose.key,value=JsonUtility.ToJson(pose)});
        }
        return result.ToArray();
    }
    public static string FirstRestChange(RestField[] before,RestField[] after)
    {
        if(before.Length!=after.Length) return "driver rest field count";
        for(int i=0;i<before.Length;i++) if(before[i].key!=after[i].key || before[i].value!=after[i].value) return after[i].key;
        return null;
    }
    public static bool InCaptureScope(Collider value,pickupable extra=null)
    { return value!=null && value.gameObject.scene.IsValid() && (value.transform.IsChildOf(car.transform) || value.attachedRigidbody==car.RB ||
        (extra!=null && value.transform.IsChildOf(extra.transform))); }
    public static Snapshot Capture(string phase,int step,string callback,HashSet<mountStuff> completed,pickupable extra=null)
    {
        // Source-generated wheel lock boxes have roles independent of their unnamed sibling paths.
        for(int index=0;index<car.whColliders.Count;index++)
        {
            var box=car.whColliders[index];keys[box.transform]="level1:77839/wheel-lock["+index+"]/transform";
            componentKeys[box]="level1:77839/wheel-lock["+index+"]/BoxCollider";
        }
        var shapes=new HashSet<Collider>(car.GetComponentsInChildren<Collider>(true));
        foreach(var value in Resources.FindObjectsOfTypeAll<Collider>()) if(InCaptureScope(value,extra)) shapes.Add(value);
        if(extra!=null) foreach(var value in extra.GetComponentsInChildren<Collider>(true)) shapes.Add(value);
        G11Lifecycle.CapturedScope(shapes,phase);
        var transforms=new HashSet<Transform>();
        foreach(var shape in shapes)
        {
            var attached=shape.attachedRigidbody;
            for(var value=shape.transform;value!=null;value=value.parent)
            { transforms.Add(value);if(attached!=null && value==attached.transform) break; }
        }
        for(var parent=car.transform;parent!=null;parent=parent.parent) transforms.Add(parent);
        var slots=car.GetComponentsInChildren<partslotscript>(true);
        var engine=car.GetComponentsInChildren<enginescript>(true).Single();
        var snapshot=new Snapshot{phase=phase,source_callback=callback,frame=Time.frameCount,physics_step=step,time=Time.time,
            fixed_time=Time.fixedTime,delta_time=Time.deltaTime,random_state=JsonUtility.ToJson(UnityEngine.Random.state),body=ReadBody(car.RB),
            nested_bodies=car.GetComponentsInChildren<Rigidbody>(true).Concat(shapes.Select(value=>value.attachedRigidbody).Where(value=>value!=null)).Distinct().Where(value=>value!=car.RB).Select(ReadBody).OrderBy(value=>value.key,StringComparer.Ordinal).ToArray(),
            colliders=shapes.Select(ReadShape).OrderBy(value=>value.key,StringComparer.Ordinal).ToArray(),
            transforms=transforms.Select(ReadPose).OrderBy(value=>value.key,StringComparer.Ordinal).ToArray(),
            cargo=locked.Where(value=>value.cargo!=null).Select(ReadCargo).ToArray(),
            mounts=car.GetComponentsInChildren<mountStuff>(true).Select(value=>new Mount{key=Key(value),parent=Key(value.transform.parent),source_parent=Key(value.parentTosave),
                id=value.id,no_physics_lock=value.noPhysicsLock,complete=completed!=null && completed.Contains(value),stored=value.stored.Where(c=>c!=null).Select(c=>Key(c.transform)).ToArray()}).ToArray(),
            slots=slots.Select((value,index)=>new Slot{key=Key(value),source_index=index,part=value.part==null?null:Key(value.part.transform),
                crafted=value.part!=null && value.part.p.crafted,condition=value.part==null?null:Key(value.part.GetComponent<partconditionscript>())}).ToArray(),
            engine=new Engine{tmove=ReadPose(engine.TMove),trot=engine.TRot.Select(ReadPose).ToArray(),engine_move=ReadPose(car.EngineMove),rpm=engine.rpm,
                ignition=car.ignition,max_vibrate=engine.maxEngineVibrate,max_rotate=engine.maxEngineRotate,max_vibrate_speed=engine.maxEngineVibrateSpeed},
            physical_ready=PhysicalReady(),readiness_failure=ReadinessFailure(),unavailable=new string[0],
            observer_body_present=G11Capture.ObservedBody!=null,observer_body_is_current=G11Capture.ObservedBody==car.RB,
            observer_body_instance_id=G11Capture.ObservedBody==null?0:G11Capture.ObservedBody.GetInstanceID()};
        snapshot.complete=snapshot.colliders.All(shape=>shape.complete);
        if(!snapshot.complete) snapshot.unavailable=snapshot.colliders.Where(shape=>!shape.complete).Select(shape=>shape.key+": "+shape.unknown_shape).ToArray();
        snapshots.Add(snapshot);return snapshot;
    }
}
