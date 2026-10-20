using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.XR;

// One held setup input. Original fpscontroller.Update owns all physical shape outputs.
public static class G11Driver
{
    [Serializable] public class Pose
    {
        public string field,key,parent;
        public Vector3 local_position,local_scale;
        public Quaternion local_rotation;
    }
    [Serializable] public class Input
    {
        public string references,parent_body;
        public Pose[] poses;
        public float yaw,pitch,stored_dt,duck_value=1f;
        public bool duck,last_ducked,stand_lean;
        public float look_x,look_y,scroll;
        public bool lean,lean_down,lean_up,recenter,recenter_down,recenter_up,key_looking,input_duck,input_duck_down,input_duck_up;
        public Vector3 parent_velocity,car_velocity_history,head_velocity_history,head_bob_history;
    }
    [Serializable] public struct Context
    {
        public bool source_player,source_main,source_car,source_seat,seat_car,seat_in_use,driver_seat,sitting,parent_matches;
        public bool car_player,car_driving,car_player_driving,driver_update_flag,mind,died,cursor_locked,multi,remote_animation,sleeping,xr;
    }
    [Serializable] public class Stamp
    {
        public string phase,callback;
        public int frame,physics_step,update_sequence,input_revision;
        public float time,fixed_time,delta_time,stored_dt,consumed_dt;
        public bool original_body_present,managed_body_present,owners_transferred,consistent;
        public string[] collider_owners;
        public Context context;
    }
    [Serializable] public class State
    {
        public string phase,references,parent_body;
        public int update_sequence,input_revision;
        public Pose[] poses;
        public Pose piss_pose;
        public Vector3 look_point,parent_velocity,car_velocity_history,head_velocity_history,head_bob_history;
        public float yaw,pitch,stored_dt,consumed_dt,duck_value,look_x,look_y,scroll;
        public bool duck,last_ducked,stand_lean,drive_head_bob,xr,mind,died,cursor_locked,neutral,driver_update_flag,sleeping;
    }
    [Serializable] public class Report
    {
        public string original_body;
        public string[] expected_colliders;
        public int original_body_instance_id,input_applications;
        public Input input;
        public List<Stamp> stamps=new List<Stamp>();
        public List<State> states=new List<State>();
    }
    public static readonly Report report=new Report();
    static fpscontroller player;
    static carscript car;
    static seatscript seat;
    static Rigidbody originalBody;
    static Collider[] transfer;
    static Transform[] transforms;
    static readonly string[] fields={"Tb","Th","THeadDir","THeadBob","THeadLean","TdefStandPos","TStandHeadPos","TProneHeadPos"};
    static bool getInExited,removed,transferred,applied,prepared,armed;
    static int ownershipUpdate,appliedUpdate,preparedUpdate,startedFrame;
    public static int UpdateSequence { get; private set; }
    public static float ConsumedDt { get; private set; }
    public static float StoredDt { get { return player==null?0f:player.dt; } }
    static string References()
    {
        return String.Join(";",transforms.Select((t,i)=>fields[i]+"="+G11Geometry.Key(t)).Concat(new[]{
            "player="+G11Geometry.Key(player),"seat="+G11Geometry.Key(seat),"sitPos="+G11Geometry.Key(seat.sitPos),
            "CBody="+G11Geometry.Key(player.CBody),"Feet2="+G11Geometry.Key(player.Feet2),
            "piss="+G11Geometry.Key(player.piss),"anim="+G11Geometry.Key(player.anim),"input="+G11Geometry.Key(player.input)}).ToArray());
    }
    static Pose ReadPose(Transform t,string field)
    {
        return new Pose{field=field,key=G11Geometry.Key(t),parent=G11Geometry.Key(t.parent),
            local_position=t.localPosition,local_rotation=t.localRotation,local_scale=t.localScale};
    }
    static Pose[] ReadPoses() { return transforms.Select((t,i)=>ReadPose(t,fields[i])).ToArray(); }
    static bool Consistent()
    {
        return mainscript.M.player==player && player.M==mainscript.M && player.Car==car && player.seat==seat &&
            seat.Car==car && seat.inUse && seat.driverSeat0 && player.Bsitting &&
            player.Tb.parent==seat.sitPos && car.player==player && car.driving && car.isPlayerDriving;
    }
    static bool OwnersTransferred() { return transfer.All(c=>c!=null && c.attachedRigidbody==car.RB); }
    static bool Neutral()
    {
        var i=player.input;
        return i.lookX==0f && i.lookY==0f && i.scroll==0f && !i.lean && !i.leanDown && !i.leanUp &&
            !i.recenter && !i.recenterDown && !i.recenterUp && !i.keyLooking && !i.duck && !i.duckDown && !i.duckUp &&
            player.LOOKX==0f && player.LOOKY==0f && player.scroll==0f;
    }
    static Context ReadContext()
    {
        return new Context{source_player=mainscript.M.player==player,source_main=player.M==mainscript.M,
            source_car=player.Car==car,source_seat=player.seat==seat,seat_car=seat.Car==car,seat_in_use=seat.inUse,
            driver_seat=seat.driverSeat0,sitting=player.Bsitting,parent_matches=player.Tb.parent==seat.sitPos,
            car_player=car.player==player,car_driving=car.driving,car_player_driving=car.isPlayerDriving,
            driver_update_flag=player.Bdriving,mind=player.mind,died=player.died,cursor_locked=mainscript.M.crsrLocked,
            multi=mainscript.M.multi,remote_animation=player.anim.gotUpd,sleeping=mainscript.M.sleeping,xr=XRSettings.enabled};
    }
    static void RequireLocal(string callback="observer.LateUpdate")
    {
        // Bdriving is cleared by player Update and restored by car Update. It is not ownership.
        if(!XRSettings.enabled && !mainscript.M.multi && !player.anim.gotUpd && player.mind && !player.died &&
            mainscript.M.crsrLocked && !mainscript.M.sleeping && Consistent()) return;
        StampAt("local-state-rejected",callback,G11Capture.ObservedPhysicsStep);
        report.states.Add(ReadState("local-state-rejected"));
        G11Capture.CaptureDriverRejection(callback);
        throw new InvalidOperationException((XRSettings.enabled?"Unexpected XR driver setup: ":"Inconsistent local source driver state: ")+
            JsonUtility.ToJson(ReadContext()));
    }
    static void StampAt(string phase,string callback,int step)
    {
        report.stamps.Add(new Stamp{phase=phase,callback=callback,frame=Time.frameCount,physics_step=step,
            time=Time.time,fixed_time=Time.fixedTime,delta_time=Time.deltaTime,stored_dt=player.dt,consumed_dt=ConsumedDt,
            update_sequence=UpdateSequence,input_revision=G11Capture.InputRevision,original_body_present=originalBody!=null,
            managed_body_present=player.RB!=null,owners_transferred=OwnersTransferred(),consistent=Consistent(),
            context=ReadContext(),collider_owners=transfer.Select(c=>c==null?"destroyed collider":G11Geometry.Key(c.attachedRigidbody)).ToArray()});
    }
    public static void Retain(fpscontroller p,carscript target,seatscript selected)
    {
        player=p;car=target;seat=selected;startedFrame=Time.frameCount;
        originalBody=player.RB;
        if(originalBody==null) throw new InvalidOperationException("Original driver Rigidbody missing before GetIn");
        transfer=player.Tb.GetComponentsInChildren<Collider>(true).Where(c=>c.attachedRigidbody==originalBody).ToArray();
        if(transfer.Length==0) throw new InvalidOperationException("Original driver collider transfer set is empty");
        transforms=new[]{p.Tb,p.Th,p.THeadDir,p.THeadBob,p.THeadLean,p.TdefStandPos,p.TStandHeadPos,p.TProneHeadPos};
        if(transforms.Any(t=>t==null) || p.CBody==null || p.Feet2==null || p.piss==null || p.anim==null || p.input==null)
            throw new InvalidOperationException("Incomplete original driver source references");
        report.original_body=G11Geometry.Key(originalBody);report.original_body_instance_id=originalBody.GetInstanceID();
        report.expected_colliders=transfer.Select(c=>G11Geometry.Key(c)).ToArray();
        StampAt("before-get-in","observer.Initialize",G11Capture.ObservedPhysicsStep);
    }
    public static void Observe(object owner,string phase,int step)
    {
        if(player==null || !ReferenceEquals(owner,player)) return;
        if(phase=="driver.get-in.exit")
        {
            getInExited=true;StampAt("get-in-exit",phase,step);return;
        }
        if(phase=="driver.update.entry") { ConsumedDt=player.dt;return; }
        if(phase!="driver.update.exit") return;
        UpdateSequence++;
        if(armed || !getInExited || !removed || !transferred) return;
        RequireLocal(phase);
        if(UpdateSequence>ownershipUpdate && !applied) StampAt("seated-update-exit",phase,step);
        if(applied && UpdateSequence>appliedUpdate)
        {
            StampAt(prepared?"held-update-exit":"stationary-update-exit",phase,step);
            report.states.Add(ReadState(prepared?"backlog-drained-held":"prepared-held"));
        }
    }
    static Input MakeInput()
    {
        var poses=ReadPoses();
        // All target positions use the existing source standing target in world space.
        Vector3 target=player.TdefStandPos.position;
        for(int i=1;i<=6;i++)
            if(i!=5) poses[i].local_position=transforms[i].parent.InverseTransformPoint(target);
        return new Input{references=References(),parent_body=G11Geometry.Key(car.RB),poses=poses,
            yaw=player.FxRot,pitch=player.FyRot,stored_dt=player.dt};
    }
    static void Apply(Input input)
    {
        RequireLocal();
        if(input==null || input.poses==null || input.poses.Length!=transforms.Length || input.references!=References() ||
            input.parent_body!=G11Geometry.Key(car.RB) || input.stored_dt<=0f || float.IsNaN(input.stored_dt) || float.IsInfinity(input.stored_dt) ||
            input.duck || input.last_ducked || input.stand_lean || input.duck_value!=1f ||
            input.look_x!=0f || input.look_y!=0f || input.scroll!=0f || input.lean || input.lean_down || input.lean_up ||
            input.recenter || input.recenter_down || input.recenter_up || input.key_looking || input.input_duck || input.input_duck_down || input.input_duck_up ||
            !input.parent_velocity.Equals(Vector3.zero) || !input.car_velocity_history.Equals(Vector3.zero) ||
            !input.head_velocity_history.Equals(Vector3.zero) || !input.head_bob_history.Equals(Vector3.zero))
            throw new InvalidOperationException("Invalid canonical stationary driver input");
        for(int i=0;i<transforms.Length;i++)
        {
            var pose=input.poses[i];var t=transforms[i];
            if(pose.field!=fields[i] || pose.key!=G11Geometry.Key(t) || pose.parent!=G11Geometry.Key(t.parent))
                throw new InvalidOperationException("Canonical driver reference or parent differs: "+fields[i]);
            t.localPosition=pose.local_position;t.localRotation=pose.local_rotation;t.localScale=pose.local_scale;
        }
        player.FxRot=input.yaw;player.FyRot=input.pitch;player.dt=input.stored_dt;player.duckValue=input.duck_value;
        player.Bduck=input.duck;player.lastDucked=input.last_ducked;player.standLean=input.stand_lean;
        player.PRBV=input.parent_velocity;player.parentRigidbody=car.RB;
        player.anim.VCarV=input.car_velocity_history;player.anim.VHeadV=input.head_velocity_history;player.anim.VHeadBob=input.head_bob_history;
        player.LOOKX=input.look_x;player.LOOKY=input.look_y;player.scroll=input.scroll;
        var controls=player.input;
        controls.lookX=input.look_x;controls.lookY=input.look_y;controls.scroll=input.scroll;
        controls.lean=input.lean;controls.leanDown=input.lean_down;controls.leanUp=input.lean_up;
        controls.recenter=input.recenter;controls.recenterDown=input.recenter_down;controls.recenterUp=input.recenter_up;
        controls.keyLooking=input.key_looking;controls.duck=input.input_duck;controls.duckDown=input.input_duck_down;controls.duckUp=input.input_duck_up;
        report.input=input;report.input_applications++;applied=true;appliedUpdate=UpdateSequence;
        StampAt("stationary-input-applied","observer.LateUpdate",G11Capture.ObservedPhysicsStep);
    }
    public static bool Tick(Input canonical,int step)
    {
        if(Time.frameCount-startedFrame>600) throw new InvalidOperationException("Driver callback or native ownership readiness timeout");
        if(!getInExited) return false;
        if(!removed && originalBody==null) { removed=true;ownershipUpdate=UpdateSequence;StampAt("native-body-removed","observer.LateUpdate",step); }
        if(removed && !transferred && OwnersTransferred())
        { transferred=true;ownershipUpdate=UpdateSequence;StampAt("collider-owners-transferred","observer.LateUpdate",step); }
        if(!removed || !transferred || UpdateSequence<=ownershipUpdate) return false;
        RequireLocal();
        if(!car.RB.isKinematic || !car.RB.velocity.Equals(Vector3.zero) || !car.RB.angularVelocity.Equals(Vector3.zero))
            throw new InvalidOperationException("Stationary driver setup requires the held source car at zero velocity");
        if(!applied) { Apply(canonical??MakeInput());return false; }
        if(!Neutral()) throw new InvalidOperationException("Stationary driver setup consumed non-neutral input");
        return UpdateSequence>appliedUpdate;
    }
    public static State ReadState(string phase)
    {
        return new State{phase=phase,references=References(),parent_body=G11Geometry.Key(player.parentRigidbody),poses=ReadPoses(),
            piss_pose=ReadPose(player.piss.transform,"piss"),look_point=player.HlookPoint,update_sequence=UpdateSequence,
            input_revision=G11Capture.InputRevision,yaw=player.FxRot,pitch=player.FyRot,stored_dt=player.dt,consumed_dt=ConsumedDt,
            duck_value=player.duckValue,duck=player.Bduck,last_ducked=player.lastDucked,stand_lean=player.standLean,
            parent_velocity=player.PRBV,car_velocity_history=player.anim.VCarV,head_velocity_history=player.anim.VHeadV,head_bob_history=player.anim.VHeadBob,
            drive_head_bob=settingsscript.s.S.BDriveHeadBob,xr=XRSettings.enabled,mind=player.mind,died=player.died,
            cursor_locked=mainscript.M.crsrLocked,driver_update_flag=player.Bdriving,sleeping=mainscript.M.sleeping,look_x=player.LOOKX,look_y=player.LOOKY,scroll=player.scroll,neutral=Neutral()};
    }
    public static void Prepared() { prepared=true;preparedUpdate=UpdateSequence; }
    public static bool NextUpdateCompleted { get { return UpdateSequence>preparedUpdate; } }
    public static void RequireHeld()
    {
        RequireLocal();
        if(originalBody!=null || player.RB!=null || !OwnersTransferred() || !Neutral())
            throw new InvalidOperationException("Driver ownership or neutral input changed at held boundary");
    }
    public static void Arm() { RequireHeld();armed=true; }
}
