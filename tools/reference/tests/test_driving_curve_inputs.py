"""Execute the one-time capture input list with a recording native-curve stand-in."""
from pathlib import Path
import subprocess
import unittest
from tools.reference.driving import method

ROOT=Path(__file__).resolve().parents[1]


class DrivingCurveInputTests(unittest.TestCase):
    def test_existing_boundaries_and_each_interval_quarter_use_evaluate(self):
        capture=(ROOT/'driving_capture.cs').read_text()
        body=method(capture,'static void CaptureCurve(')[0]
        code=STUBS+'public static class Capture {'+CAPTURE+body+CHECKS+'}'
        command="Add-Type -TypeDefinition @'\n"+code+"\n'@\n[Capture]::Run()"
        result=subprocess.run(['pwsh','-NoProfile','-Command',command],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('G11_CURVE_INPUTS_PASS',result.stdout)


STUBS=r"""
using System;
using System.Collections.Generic;
using System.Linq;
public struct Keyframe {public float time;public Keyframe(float time){this.time=time;}}
public class AnimationCurve {
    public Keyframe[] keys; public int length {get{return keys.Length;}}
    public int preWrapMode=3,postWrapMode=4;
    public List<float> evaluated=new List<float>();
    public float Evaluate(float input){evaluated.Add(input);return input*2+7;}
}
"""
CAPTURE=r"""
    public class Key {public float time;public Key(Keyframe source){time=source.time;}}
    public class Curve {public string owner,name;public Key[] keys;public int pre_wrap,post_wrap;public float[] inputs,values;}
    public class Result {public List<Curve> curves=new List<Curve>();}
    static Result result=new Result();
"""
CHECKS=r"""
    public static void Run(){
        var source=new AnimationCurve{keys=new[]{new Keyframe(-3),new Keyframe(5),new Keyframe(17)}};
        CaptureCurve("source-owner","source-curve",source);
        var row=result.curves.Single();
        float[] old={0,-23,-3,5,17,37};float[] interior={-1,1,3,8,11,14};
        if(!row.inputs.Take(old.Length).SequenceEqual(old) || !row.inputs.Skip(old.Length).SequenceEqual(interior))throw new Exception("source interval coverage changed");
        if(!source.evaluated.SequenceEqual(row.inputs) || !row.values.SequenceEqual(row.inputs.Select(x=>x*2+7)))throw new Exception("native Evaluate input/output link missing");
        if(row.owner!="source-owner" || row.name!="source-curve" || row.pre_wrap!=3 || row.post_wrap!=4 || !row.keys.Select(x=>x.time).SequenceEqual(source.keys.Select(x=>x.time)))throw new Exception("source curve provenance changed");
        Console.WriteLine("G11_CURVE_INPUTS_PASS");
    }
"""

if __name__=='__main__':unittest.main()
