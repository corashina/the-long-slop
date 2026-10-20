using System;
using System.Collections.Generic;
using System.Threading;
using UnityEngine;

// Every original setter receives its exact target and value once. Holds belong to the observer.
public static class G11BodyTrace
{
    [Serializable] public struct Body
    {
        public string key;
        public int instance_id;
        public bool present,kinematic;
        public Vector3 velocity,angular_velocity;
    }
    [Serializable] public struct Event
    {
        public string kind,callback,caller,original_opcode,original_exception;
        public int order,call_id,il_offset,frame,physics_step,input_revision,update_sequence,input_applications;
        public float time,fixed_time,delta_time;
        public bool requested,armed,current_is_retained,current_is_observer,original_hold,copy_hold;
        public Body target,retained,current,observer,copied;
    }
    [Serializable] public class Report
    {
        public string retained_body,copied_body,hold_failure;
        public bool isolated,ended;
        public int retained_instance_id,copied_instance_id;
        public List<Event> events=new List<Event>(32768);
    }
    public static readonly Report report=new Report();
    static carscript car;
    static Rigidbody retained;
    static bool bound,originalHeld,copyHeld,prepared,backlog,releaseReady,released;
    static Rigidbody copied;
    static int order,callId;
    public static void Bind(carscript target)
    {
        if(bound) throw new InvalidOperationException("Retained body ownership cannot be rebound");
        car=target;retained=target.RB;
        report.retained_body=G11Geometry.Key(retained);
        report.retained_instance_id=retained.GetInstanceID();bound=true;
    }
    static Body Read(Rigidbody body)
    {
        bool present=body!=null;
        return new Body{key=ReferenceEquals(body,retained)?report.retained_body:G11Geometry.Key(body),
            instance_id=ReferenceEquals(body,retained)?report.retained_instance_id:present?body.GetInstanceID():0,
            present=present,kinematic=present && body.isKinematic,
            velocity=present?body.velocity:Vector3.zero,angular_velocity=present?body.angularVelocity:Vector3.zero};
    }
    static void Record(string kind,string callback,Rigidbody target,bool requested,string caller,int offset,string opcode,int call,string error)
    {
        if(report.events.Count==report.events.Capacity) throw new InvalidOperationException("Selected body trace capacity exceeded");
        var current=car.RB;var observer=G11Capture.ObservedBody;
        report.events.Add(new Event{kind=kind,callback=callback,caller=caller,il_offset=offset,original_opcode=opcode,
            order=Interlocked.Increment(ref order),call_id=call,frame=Time.frameCount,physics_step=G11Capture.ObservedPhysicsStep,
            input_revision=G11Capture.InputRevision,update_sequence=G11Driver.UpdateSequence,input_applications=G11Driver.InputApplications,
            time=Time.time,fixed_time=Time.fixedTime,delta_time=Time.deltaTime,requested=requested,armed=G11Capture.IsArmed,
            original_hold=originalHeld,copy_hold=copyHeld,target=Read(target),retained=Read(retained),current=Read(current),observer=Read(observer),copied=Read(copied),
            current_is_retained=ReferenceEquals(current,retained),current_is_observer=ReferenceEquals(current,observer),original_exception=error});
    }
    static Exception TryRecord(string kind,Rigidbody target,bool requested,string caller,int offset,string opcode,int call,string error)
    {
        try { Record(kind,offset<0?opcode:"Rigidbody.set_isKinematic",target,requested,caller,offset,offset<0?null:opcode,call,error);return null; }
        catch(Exception failure) { return failure; }
    }
    static void Remember(Exception error)
    {
        if(report.hold_failure==null) report.hold_failure=error.ToString();
    }
    static bool Stationary(Body value)
    {
        return value.present && value.kinematic && value.velocity.x==0f && value.velocity.y==0f && value.velocity.z==0f &&
            value.angular_velocity.x==0f && value.angular_velocity.y==0f && value.angular_velocity.z==0f;
    }
    static void Check()
    {
        try
        {
            if(!ReferenceEquals(car.RB,retained) || retained==null) throw new InvalidOperationException("Retained body ownership changed");
            if(originalHeld && !Stationary(Read(retained))) throw new InvalidOperationException("Original body hold violated");
            if(copyHeld && !Stationary(Read(copied))) throw new InvalidOperationException("Copied body hold violated");
            if(report.hold_failure!=null) throw new InvalidOperationException(report.hold_failure);
        }
        catch(Exception error) { Remember(error);throw; }
    }
    public static void HoldOriginal()
    {
        if(!bound || originalHeld || released) throw new InvalidOperationException("Original hold registration order differs");
        retained.isKinematic=true;originalHeld=true;
        Record("hold.started","observer.original.hold",retained,true,null,-1,null,0,null);Check();
    }
    public static void HoldCopy(Rigidbody target)
    {
        if(!originalHeld || copied!=null || ReferenceEquals(target,retained) || released)
            throw new InvalidOperationException("Copied hold registration order or role differs");
        Check();copied=target;report.isolated=true;
        report.copied_body=G11Geometry.Key(copied);report.copied_instance_id=copied.GetInstanceID();
        Record("copy.created","observer.copy.created",copied,false,null,-1,null,0,null);
        copied.isKinematic=true;copyHeld=true;
        Record("hold.started","observer.copy.hold",copied,true,null,-1,null,0,null);Check();
    }
    public static void SetupBoundary(string phase)
    {
        Check();
        if(phase=="prepared-held") prepared=true;
        if(phase=="backlog-drained-held")
        {
            if(!prepared) throw new InvalidOperationException("Held preparation is missing");
            backlog=true;
        }
        if(phase=="before-dynamic")
        {
            if(!prepared || !backlog || released) throw new InvalidOperationException("Premature body release boundary");
            releaseReady=true;
        }
        Record("hold.boundary",phase,retained,false,null,-1,null,0,null);
    }
    public static void ReleaseMeasurement(Rigidbody target)
    {
        if(!releaseReady || released || !ReferenceEquals(target,copied??retained))
            throw new InvalidOperationException("Premature body release or wrong body role");
        Check();
        Record("hold.release.before","observer.before-dynamic",target,false,null,-1,null,0,null);
        target.isKinematic=false;
        if(copied!=null) copyHeld=false;else originalHeld=false;
        released=true;
        Record("hold.release.after","observer.before-dynamic",target,false,null,-1,null,0,null);Check();
    }
    public static void BeforeSolve()
    {
        if(!bound || !G11Capture.TraceOpen) return;
        Record("hold.solve","observer.FixedUpdate",retained,false,null,-1,null,0,null);Check();
    }
    public static void End()
    {
        Check();
        if(!released) throw new InvalidOperationException("Measurement hold was not released");
        Record("hold.end","observer.capture.end",retained,false,null,-1,null,0,null);report.ended=true;
    }
    public static void Set(Rigidbody target,bool requested,string caller,int offset,string opcode)
    {
        // ReferenceEquals performs no Unity body inspection for unrelated or unbound targets.
        if(!bound || !G11Capture.TraceOpen || !ReferenceEquals(target,retained)) { target.isKinematic=requested;return; }
        int call=++callId;
        var before=TryRecord("setter.before",target,requested,caller,offset,opcode,call,null);
        if(originalHeld)
            try { Check(); } catch(Exception violation) { before=before??violation; }
        try { target.isKinematic=requested; }
        catch(Exception original)
        {
            var failed=TryRecord("setter.exception",target,requested,caller,offset,opcode,call,original.ToString());
            if(before!=null || failed!=null)
                try { G11Capture.TraceFailure(before??failed); }
                catch(Exception reporting) { Debug.LogException(reporting); }
            throw;
        }
        var after=TryRecord("setter.after",target,requested,caller,offset,opcode,call,null);
        Exception restoration=null;
        if(originalHeld && !requested)
        {
            restoration=TryRecord("hold.restore.before",target,true,"observer.original.restore",-1,"observer.original.restore",call,null);
            try { target.isKinematic=true; }
            catch(Exception error) { restoration=restoration??error; }
            restoration=restoration??TryRecord("hold.restore.after",target,true,"observer.original.restore",-1,"observer.original.restore",call,null);
        }
        if(before!=null || after!=null || restoration!=null) G11Capture.TraceFailure(before??after??restoration);
    }
    public static void Boundary(string callback)
    {
        if(!bound || !G11Capture.TraceOpen) return;
        Record("body.boundary",callback,retained,false,null,-1,null,0,null);
    }
}
