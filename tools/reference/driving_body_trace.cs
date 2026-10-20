using System;
using System.Collections.Generic;
using System.Threading;
using UnityEngine;

// Evidence only. Every original setter receives its exact target and value once.
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
        public bool requested,armed,current_is_retained,current_is_observer;
        public Body target,retained,current,observer;
    }
    [Serializable] public class Report
    {
        public string retained_body;
        public int retained_instance_id;
        public List<Event> events=new List<Event>(32768);
    }
    public static readonly Report report=new Report();
    static carscript car;
    static Rigidbody retained;
    static bool bound;
    static int order,callId;
    public static void Bind(carscript target)
    {
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
            target=Read(target),retained=Read(retained),current=Read(current),observer=Read(observer),
            current_is_retained=ReferenceEquals(current,retained),current_is_observer=ReferenceEquals(current,observer),original_exception=error});
    }
    static Exception TryRecord(string kind,Rigidbody target,bool requested,string caller,int offset,string opcode,int call,string error)
    {
        try { Record(kind,offset<0?opcode:"Rigidbody.set_isKinematic",target,requested,caller,offset,offset<0?null:opcode,call,error);return null; }
        catch(Exception failure) { return failure; }
    }
    public static void Set(Rigidbody target,bool requested,string caller,int offset,string opcode)
    {
        // ReferenceEquals performs no Unity body inspection for unrelated or unbound targets.
        if(!bound || !G11Capture.TraceOpen || !ReferenceEquals(target,retained)) { target.isKinematic=requested;return; }
        int call=++callId;
        var before=TryRecord("setter.before",target,requested,caller,offset,opcode,call,null);
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
        if(before!=null || after!=null) G11Capture.TraceFailure(before??after);
    }
    public static void Boundary(string callback)
    {
        if(!bound || !G11Capture.TraceOpen) return;
        Record("body.boundary",callback,retained,false,null,-1,null,0,null);
    }
}
