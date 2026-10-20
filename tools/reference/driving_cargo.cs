using System;
using System.Collections.Generic;
using UnityEngine;

// Declared setup inputs only. Native observations remain separate from setter arguments.
public static class G11Cargo
{
    [Serializable] public class Target
    {
        public string key,pickup,save,parent,mount,lock_parent,body;
        public int category,item,lock_id;
        public Vector3 local_position,local_scale;
        public Quaternion local_rotation;
    }
    [Serializable] public class Input
    {
        public string phase="source-physical-ready";
        public int frame,physics_step,input_revision;
        public Target[] targets;
    }
    public class Binding
    {
        public Target state; public Transform root;
        public int root_id,pickup_id,save_id,mount_id,body_id,lock_parent_id,expected_lock_parent_id;
        public int[] owner_ids;
        public bool locked,body_retired,ownership_ready,unsupported;
    }
    [Serializable] public class Observation
    {
        public string key;
        public int root_id,pickup_id,save_id,mount_id,body_id,lock_parent_id;
        public int[] owner_ids;
        public Vector3 local_position,local_scale;
        public Quaternion local_rotation;
    }
    [Serializable] public class Report
    {
        public int applications,frame,physics_step,input_revision;
        public float time,fixed_time;
        public string phase;
        public Input input;
        public Target[] arguments;
        public Observation[] before,after;
        public string[] caches_before,caches_after;
    }
    public static readonly Report report=new Report();
    static Target Copy(Target value)
    {
        return new Target{key=value.key,pickup=value.pickup,save=value.save,parent=value.parent,mount=value.mount,
            lock_parent=value.lock_parent,body=value.body,category=value.category,item=value.item,lock_id=value.lock_id,
            local_position=value.local_position,local_rotation=value.local_rotation,local_scale=value.local_scale};
    }
    public static Input Declare(Binding[] cohort,int step,int revision)
    {
        var targets=new Target[cohort.Length];
        for(int index=0;index<targets.Length;index++) targets[index]=Copy(cohort[index].state);
        return new Input{frame=Time.frameCount,physics_step=step,input_revision=revision,targets=targets};
    }
    static bool Finite(float value) { return !Single.IsNaN(value) && !Single.IsInfinity(value); }
    static bool Finite(Vector3 value) { return Finite(value.x) && Finite(value.y) && Finite(value.z); }
    static bool Valid(Target value)
    {
        return value!=null && !String.IsNullOrEmpty(value.key) && !String.IsNullOrEmpty(value.pickup) &&
            !String.IsNullOrEmpty(value.save) && !String.IsNullOrEmpty(value.parent) && !String.IsNullOrEmpty(value.mount) &&
            !String.IsNullOrEmpty(value.lock_parent) && !String.IsNullOrEmpty(value.body) &&
            Finite(value.local_position) && Finite(value.local_scale) && Finite(value.local_rotation.x) &&
            Finite(value.local_rotation.y) && Finite(value.local_rotation.z) && Finite(value.local_rotation.w);
    }
    static bool SameOwner(Target left,Target right)
    {
        return left.key==right.key && left.pickup==right.pickup && left.save==right.save && left.parent==right.parent &&
            left.mount==right.mount && left.lock_parent==right.lock_parent && left.body==right.body &&
            left.category==right.category && left.item==right.item && left.lock_id==right.lock_id;
    }
    static Observation Read(Binding value)
    {
        return new Observation{key=value.state.key,root_id=value.root_id,pickup_id=value.pickup_id,save_id=value.save_id,
            mount_id=value.mount_id,body_id=value.body_id,lock_parent_id=value.lock_parent_id,owner_ids=value.owner_ids,
            local_position=value.root.localPosition,local_rotation=value.root.localRotation,local_scale=value.root.localScale};
    }
    public static void Apply(Input input,Binding[] cohort,Report output,int step,int revision,bool held)
    {
        if(output.applications!=0) throw new InvalidOperationException("Declared cargo input must apply exactly once");
        if(!held || input==null || input.phase!="source-physical-ready" || input.frame<0 || input.physics_step<0 || input.input_revision<0 ||
           input.targets==null || cohort==null || input.targets.Length!=cohort.Length)
            throw new InvalidOperationException("Missing held declared cargo cohort or source provenance");
        var byKey=new Dictionary<string,Binding>(StringComparer.Ordinal);
        var roots=new HashSet<Transform>();var pickups=new HashSet<string>();var saves=new HashSet<string>();
        foreach(var value in cohort)
        {
            if(value==null || !Valid(value.state) || value.root==null || !roots.Add(value.root) ||
               byKey.ContainsKey(value.state.key) || !pickups.Add(value.state.pickup) || !saves.Add(value.state.save) ||
               G11Geometry.Key(value.root)!=value.state.key || G11Geometry.Key(value.root.parent)!=value.state.parent ||
               !value.locked || !value.body_retired || !value.ownership_ready || value.unsupported ||
               value.lock_parent_id!=value.expected_lock_parent_id || value.root_id==0 || value.pickup_id==0 ||
               value.save_id==0 || value.mount_id==0 || value.body_id==0 || value.owner_ids==null)
                throw new InvalidOperationException("Unsupported or invalid retained cargo ownership");
            foreach(int owner in value.owner_ids) if(owner!=value.body_id)
                throw new InvalidOperationException("Declared cargo native attached body differs");
            byKey.Add(value.state.key,value);
        }
        var selected=new Binding[input.targets.Length];var seen=new HashSet<string>();
        for(int index=0;index<input.targets.Length;index++)
        {
            var target=input.targets[index];Binding value;
            if(!Valid(target) || !seen.Add(target.key) || !byKey.TryGetValue(target.key,out value) || !SameOwner(target,value.state))
                throw new InvalidOperationException("Declared cargo identity, parent, or lock owner differs");
            selected[index]=value;
        }
        output.before=new Observation[selected.Length];output.after=new Observation[selected.Length];
        output.arguments=new Target[selected.Length];
        for(int index=0;index<selected.Length;index++) { output.before[index]=Read(selected[index]);output.arguments[index]=Copy(input.targets[index]); }
        output.input=input;output.phase="source-physical-ready";output.frame=Time.frameCount;output.physics_step=step;
        output.input_revision=revision;output.time=Time.time;output.fixed_time=Time.fixedTime;output.applications++;
        for(int index=0;index<selected.Length;index++)
        {
            var root=selected[index].root;var target=output.arguments[index];
            root.localPosition=target.local_position;
            root.localRotation=target.local_rotation;
            root.localScale=target.local_scale;
            output.after[index]=Read(selected[index]);
        }
    }
}
