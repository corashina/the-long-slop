using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

// Setup observations only. Native source callbacks and destruction remain authoritative.
public static class G11Lifecycle
{
    [Serializable] public class Mark
    {
        public string callback; public int order,frame,update_sequence,input_revision; public float time;
    }
    [Serializable] public class Scope
    {
        public Mark observed;
        public bool member,owner_is_null;
        public string parent,owner;
        public int owner_id;
    }
    [Serializable] public class Audio
    {
        public string prefab,clip,creation_parent,parent,source_owner,source_stack,component,collider;
        public int object_id,component_id,collider_id,prefab_id,clip_id;
        public float clip_length,destroy_delay;
        public Mark created,destruction_requested,parent_observed,removed;
        public bool original_destroy_requested,object_null,component_null,collider_null,required,scope_member,created_after_close;
        public List<Scope> membership=new List<Scope>();
    }
    [Serializable] public class Ray
    {
        public Vector3 origin,direction,point; public float distance; public int mask;
        public bool hit; public string collider; public int collider_id;
    }
    [Serializable] public class Update
    {
        public int sequence,setup_revision,look_producer,midpoint_producer;
        public Mark entry,forward,look_entry,look_exit,look_output,midpoint,exit;
        public Vector3 entry_look,entry_piss,entry_piss_forward,entry_head,entry_head_forward,consumed_look,consumed_piss,produced_look,midpoint_position;
        public Quaternion forward_rotation,look_rotation;
        public G11Driver.BodyState entry_body,exit_body;
        public Vector3 entry_parent_velocity,exit_parent_velocity,entry_car_velocity_history,exit_car_velocity_history,
            entry_head_velocity_history,exit_head_velocity_history,entry_head_bob_history,exit_head_bob_history;
        public float entry_stored_dt,exit_stored_dt,delta_time;
        public List<Ray> rays=new List<Ray>();
        public G11Geometry.RestField[] rest_fields;
        public bool dependency_complete,rest_equal;
        public string first_change;
    }
    [Serializable] public class Report
    {
        public bool begun,owners_complete,cohort_closed,ready,armed;
        public Mark opened,owners_completed,closed,prepared;
        public string owner_completion_evidence;
        public List<Mark> owner_callbacks=new List<Mark>();
        public List<Audio> audio=new List<Audio>();
        public List<Audio> new_after_close=new List<Audio>();
        public List<Update> updates=new List<Update>();
    }
    class Retained { public GameObject obj;public AudioSource source;public Collider collider;public Audio record; }
    class Owner { public object value; public string callback; }
    static readonly List<Retained> retained=new List<Retained>();
    static readonly List<Owner> owners=new List<Owner>();
    public static readonly Report report=new Report();
    static Update current,last;
    static int order;
    static fpscontroller player;
    static int lookProducer,midpointProducer;
    static Vector3 lookValue,midpointValue;
    static Collider[] driverShapes;
    static Transform[] driverTransforms;
    public static Mark At(string callback,int sequence=0)
    { return new Mark{callback=callback,order=++order,frame=Time.frameCount,time=Time.time,update_sequence=sequence,input_revision=G11Capture.InputRevision}; }
    public static void Begin() { report.begun=true;report.opened=At("world.entry"); }
    public static void OwnerBoundary(object owner,string phase)
    {
        if(!report.begun || report.armed) return;
        if(phase=="setup.crafted.entry" || phase=="setup.slot.entry") owners.Add(new Owner{value=owner,callback=phase});
        if(phase=="setup.crafted.exit" || phase=="slot.ready")
        {
            if(owners.Count==0 || !ReferenceEquals(owners[owners.Count-1].value,owner)) throw new InvalidOperationException("Unbalanced original setup owner callback: "+phase);
            owners.RemoveAt(owners.Count-1);
        }
        if(phase=="setup.crafted.exit" || phase=="slot.ready" || phase=="storage.ready" || phase=="world.exit")
            report.owner_callbacks.Add(At(phase+":"+(owner is Component?G11Geometry.Key((Component)owner):owner.GetType().Name)));
    }
    public static void Created(GameObject obj,AudioClip clip)
    {
        if(!report.begun || report.armed) return;
        var source=obj.GetComponent<AudioSource>();var collider=obj.GetComponent<Collider>();
        if(source==null || collider==null || clip==null) throw new InvalidOperationException("Incomplete actual source M.Audio clone references");
        string stack=new System.Diagnostics.StackTrace().ToString();
        var owner=owners.Count==0?null:owners[owners.Count-1];
        var audio=new Audio{prefab="sharedassets1.assets:21250",prefab_id=mainscript.M.Audio.GetInstanceID(),clip=clip.name,clip_id=clip.GetInstanceID(),
            clip_length=clip.length,object_id=obj.GetInstanceID(),component_id=source.GetInstanceID(),collider_id=collider.GetInstanceID(),
            component="native-instance:"+source.GetInstanceID(),collider="native-instance:"+collider.GetInstanceID(),
            source_owner=owner==null?"original source stack":G11Geometry.Key((Component)owner.value),
            source_stack=stack,created=At(owner==null?"mainscript.PlayClipAtPoint.Instantiate":owner.callback)};
        string parent=G11Geometry.Key(obj.transform.parent);
        audio.creation_parent=parent;audio.parent=parent;
        audio.parent_observed=At("mainscript.PlayClipAtPoint.Instantiate.parent");
        audio.created_after_close=report.cohort_closed;
        report.audio.Add(audio);var value=new Retained{obj=obj,source=source,collider=collider,record=audio};retained.Add(value);
        ObserveScope(value,G11Geometry.InCaptureScope(collider),"mainscript.PlayClipAtPoint.Instantiate",true);
    }
    public static void DestroyRequested(UnityEngine.Object obj,float delay)
    {
        if(!report.begun || report.armed) return;
        var value=retained.SingleOrDefault(r=>r.obj==obj);
        if(value==null) throw new InvalidOperationException("Source audio destruction has no retained creation reference");
        value.record.original_destroy_requested=true;value.record.destroy_delay=delay;
        value.record.destruction_requested=At("mainscript.PlayClipAtPoint.Destroy(Object,clip.length)");
        if(value.source.clip==null || value.source.clip.GetInstanceID()!=value.record.clip_id || delay!=value.record.clip_length)
            throw new InvalidOperationException("Source audio destruction clip reference or original delay differs");
    }
    public static void OwnersCompleted(bool complete,string evidence)
    {
        if(!complete || owners.Count!=0) throw new InvalidOperationException("Original setup owners have not completed");
        report.owners_complete=true;report.owner_completion_evidence=evidence;
        report.owners_completed=At("observer.Initialize.explicit-setup-exit");
    }
    static void ObserveScope(Retained value,bool member,string callback,bool reject)
    {
        var a=value.record;
        string parent=value.obj==null?a.parent:G11Geometry.Key(value.obj.transform.parent);
        var owner=value.collider==null?null:value.collider.attachedRigidbody;
        string ownerKey=G11Geometry.Key(owner);int ownerId=owner==null?0:owner.GetInstanceID();
        var previous=a.membership.Count==0?null:a.membership[a.membership.Count-1];
        if(previous==null || previous.member!=member || previous.parent!=parent || previous.owner_id!=ownerId)
            a.membership.Add(new Scope{observed=At(callback+".scope"),member=member,parent=parent,owner=ownerKey,owner_id=ownerId,owner_is_null=owner==null});
        a.scope_member=member;
        if(!member || a.required) return;
        a.required=true;
        if(report.cohort_closed)
        {
            report.new_after_close.Add(a);
            if(reject) throw new InvalidOperationException("New captured-geometry audio membership after cohort closure: "+JsonUtility.ToJson(a));
        }
    }
    public static void CapturedScope(HashSet<Collider> shapes,string phase)
    {
        if(!report.begun || report.armed) return;
        foreach(var value in retained) if(value.collider!=null && shapes.Contains(value.collider)) ObserveScope(value,true,"geometry."+phase,false);
    }
    public static void PollAudio(string callback)
    {
        if(!report.begun || report.armed) return;
        foreach(var value in retained)
        {
            var a=value.record;
            ObserveScope(value,G11Geometry.InCaptureScope(value.collider),callback,true);
            if(value.obj!=null)
            {
                string parent=G11Geometry.Key(value.obj.transform.parent);
                if(a.parent_observed==null || a.parent!=parent) { a.parent=parent;a.parent_observed=At(callback+".parent"); }
            }
            a.object_null=value.obj==null;a.component_null=value.source==null;a.collider_null=value.collider==null;
            if(a.object_null && a.component_null && a.collider_null && a.removed==null) a.removed=At(callback+".Unity-null");
        }
    }
    public static Audio RetiredCargoCollider(Collider collider,int instanceId)
    {
        // Unity-null alone is insufficient. Match the retained managed reference and its live native identity.
        if(ReferenceEquals(collider,null) || collider!=null) return null;
        var value=retained.SingleOrDefault(r=>ReferenceEquals(r.collider,collider) && r.record.collider_id==instanceId);
        if(value==null) return null;
        var a=value.record;
        if(value.obj!=null || value.source!=null || value.collider!=null ||
            !a.required || a.created_after_close || !a.original_destroy_requested ||
            !a.object_null || !a.component_null || !a.collider_null ||
            a.prefab!="sharedassets1.assets:21250" || a.object_id==0 || a.component_id==0 || a.collider_id==0 ||
            a.prefab_id==0 || a.clip_id==0 || String.IsNullOrEmpty(a.source_owner) || String.IsNullOrEmpty(a.source_stack) ||
            a.destroy_delay!=a.clip_length || a.created==null || a.destruction_requested==null || a.parent_observed==null || a.removed==null ||
            a.destruction_requested.callback!="mainscript.PlayClipAtPoint.Destroy(Object,clip.length)" ||
            (a.removed.callback!="driver.update.exit.Unity-null" && a.removed.callback!="observer.LateUpdate.Unity-null") ||
            a.created.order>=a.destruction_requested.order || a.destruction_requested.order>=a.removed.order ||
            a.parent_observed.order>=a.removed.order || !a.membership.Any(s=>s.member)) return null;
        return a;
    }
    public static bool AudioReady()
    {
        return report.owners_complete && retained.Any(r=>r.record.required) && retained.Where(r=>r.record.required).All(r=>r.record.original_destroy_requested && r.record.removed!=null);
    }
    public static string Pending()
    {
        var live=retained.FirstOrDefault(r=>r.record.required && r.record.removed==null);
        if(live!=null) return "live original audio reference "+JsonUtility.ToJson(live.record);
        if(last==null) return "no completed post-setup original player Update";
        if(!last.dependency_complete) return "unconsumed source producer "+JsonUtility.ToJson(last);
        return "first changing driver field "+last.first_change+"; "+JsonUtility.ToJson(last);
    }
    public static void Applied(fpscontroller p)
    {
        player=p;driverShapes=p.Tb.GetComponentsInChildren<Collider>(true);
        var transforms=new HashSet<Transform>();
        foreach(var c in driverShapes) for(var t=c.transform;t!=null;t=t.parent) transforms.Add(t);
        driverTransforms=transforms.OrderBy(G11Geometry.Key,StringComparer.Ordinal).ToArray();
    }
    public static void Player(fpscontroller p,string phase,int sequence,int setupRevision)
    {
        if(report.armed || player==null || !ReferenceEquals(p,player)) return;
        if(phase=="driver.update.entry")
        {
            current=new Update{sequence=sequence,setup_revision=setupRevision,entry=At(phase,sequence),entry_look=p.HlookPoint,
                entry_piss=p.piss.transform.position,entry_piss_forward=p.piss.transform.forward,entry_head=p.Th.position,entry_head_forward=p.Th.forward,
                entry_body=G11Driver.ReadBody(phase,sequence),entry_parent_velocity=p.PRBV,entry_car_velocity_history=p.anim.VCarV,
                entry_head_velocity_history=p.anim.VHeadV,entry_head_bob_history=p.anim.VHeadBob,entry_stored_dt=p.dt,delta_time=Time.deltaTime};
            report.updates.Add(current);return;
        }
        if(current==null) return;
        if(phase=="driver.forward.before")
        {
            current.forward=At(phase,sequence);current.consumed_look=p.HlookPoint;current.consumed_piss=p.piss.transform.position;
            current.look_producer=lookProducer;current.midpoint_producer=midpointProducer;
            current.dependency_complete=lookProducer>setupRevision && midpointProducer>setupRevision &&
                lookProducer<sequence && midpointProducer<sequence && p.HlookPoint.Equals(lookValue) && p.piss.transform.position.Equals(midpointValue);
        }
        if(phase=="driver.forward.after") current.forward_rotation=p.piss.transform.rotation;
        if(phase=="driver.look-point.entry") current.look_entry=At(phase,sequence);
        if(phase=="driver.look-point.exit")
        { current.look_exit=At(phase,sequence);current.produced_look=p.HlookPoint;lookProducer=sequence;lookValue=p.HlookPoint; }
        if(phase=="driver.look.exit") { current.look_output=At(phase,sequence);current.look_rotation=p.Th.localRotation; }
        if(phase=="driver.midpoint.after")
        { current.midpoint=At(phase,sequence);current.midpoint_position=p.piss.transform.position;midpointProducer=sequence;midpointValue=current.midpoint_position; }
        if(phase!="driver.update.exit") return;
        PollAudio("driver.update.exit");current.exit=At(phase,sequence);
        current.exit_body=G11Driver.ReadBody(phase,sequence);current.exit_parent_velocity=p.PRBV;
        current.exit_car_velocity_history=p.anim.VCarV;current.exit_head_velocity_history=p.anim.VHeadV;
        current.exit_head_bob_history=p.anim.VHeadBob;current.exit_stored_dt=p.dt;
        if(current.forward==null || current.look_entry==null || current.look_exit==null || current.look_output==null || current.midpoint==null || current.rays.Count==0)
            throw new InvalidOperationException("Incomplete original player writer observations: "+JsonUtility.ToJson(current));
        current.rest_fields=G11Geometry.DriverRest(driverShapes,driverTransforms);
        current.first_change=last==null?"first completed Update":G11Geometry.FirstRestChange(last.rest_fields,current.rest_fields);
        current.rest_equal=last!=null && current.first_change==null;
        last=current;current=null;
    }
    public static void RayObserved(Vector3 origin,Vector3 direction,float distance,int mask,bool hit,RaycastHit info)
    {
        if(current==null || current.look_entry==null || current.look_exit!=null) return;
        current.rays.Add(new Ray{origin=origin,direction=direction,distance=distance,mask=mask,hit=hit,
            collider=hit?G11Geometry.Key(info.collider):"explicit miss",collider_id=hit?info.collider.GetInstanceID():0,point=hit?info.point:Vector3.zero});
    }
    public static bool Ready { get { return AudioReady() && last!=null && last.dependency_complete && last.rest_equal &&
        retained.Where(r=>r.record.required).All(r=>r.record.removed.order<last.exit.order); } }
    public static void Prepared()
    {
        if(!Ready || owners.Count!=0) throw new InvalidOperationException("Source lifecycle is not ready: "+Pending());
        report.cohort_closed=true;report.closed=At("observer.prepared-held.close-cohort",last.sequence);
        report.ready=true;report.prepared=At("observer.prepared-held",last.sequence);
    }
    public static void RequireHeld()
    { if(!AudioReady() || !report.ready || last==null || !last.rest_equal || !last.dependency_complete || report.new_after_close.Count!=0) throw new InvalidOperationException("Source lifecycle changed at held boundary: "+Pending()); }
    public static void Arm() { RequireHeld();report.armed=true; }
}
