using System;
using UnityEngine;

// Original input callbacks consume these declarations. No source output is written here.
public static class G11Controls
{
    [Serializable] public class GearPress { public int value; }
    [Serializable] public struct Mode
    {
        public bool auto_clutch,auto_shift,force_manual,force_automatic_transmission,h_shifter;
        public bool analog_gas,analog_brake,analog_clutch,analog_steer,analog_handbrake,mouse_drive;
    }
    [Serializable] public struct State
    {
        public int input_revision,input_gears,car_gears;
        public bool input_handbrake_press,car_handbrake_press,stored_handbrake;
        public bool input_ignition_held,input_ignition_down,input_ignition_up,car_ignition_held,car_ignition_down;
        public bool seat_driver0,seat_driver1,driving,driving2,player_driving,player_driving2;
        public float input_throttle,input_brake,input_clutch,input_steer,input_handbrake;
        public float car_throttle,car_brake,car_clutch,car_steer,car_handbrake;
        public float selected_ratio,differential_ratio,brake_power,max_handbrake;
        public Mode mode;
    }
    static G11Capture.Control[] rows;
    public static void Initialize(G11Capture.Control[] controls) { rows=controls; }
    static int Index(int frame)
    {
        int selected=0;
        for(int index=1;index<rows.Length;index++) if(rows[index].frame<=frame) selected=index;
        return selected;
    }
    public static float Axis(inputscript.IN action,int frame)
    {
        var row=rows[Index(frame)];
        switch(action)
        {
            case inputscript.IN.gas:return row.throttle;
            case inputscript.IN.brake:return row.brake;
            case inputscript.IN.clutch:return row.clutch;
            case inputscript.IN.steerleft:return Mathf.Max(0f,-row.steer);
            case inputscript.IN.steerright:return Mathf.Max(0f,row.steer);
            default:return 0f;
        }
    }
    public static bool Held(inputscript.IN action,int frame)
    {
        if(action==inputscript.IN.ignition) return rows[Index(frame)].ignition_button_held;
        return Axis(action,frame)>0f;
    }
    public static bool Down(inputscript.IN action,int frame)
    {
        int index=Index(frame);var row=rows[index];
        if(frame!=row.frame) return false;
        if(action==inputscript.IN.handbrake) return row.handbrake_press;
        if(action==inputscript.IN.ignition) return row.ignition_button_held && (index==0 || !rows[index-1].ignition_button_held);
        if(row.gear_press==null) return false;
        if(action==inputscript.IN.gearr) return row.gear_press.value==-1;
        if(action==inputscript.IN.gearn) return row.gear_press.value==0;
        if(action>=inputscript.IN.gear1 && action<=inputscript.IN.gear6)
            return row.gear_press.value==(int)action-(int)inputscript.IN.gear1+1;
        return false;
    }
    public static bool Up(inputscript.IN action,int frame)
    {
        int index=Index(frame);var row=rows[index];
        return action==inputscript.IN.ignition && frame==row.frame && index>0 && rows[index-1].ignition_button_held && !row.ignition_button_held;
    }
    public static Mode ReadMode(carscript car)
    {
        var settings=settingsscript.s.S;
        return new Mode { auto_clutch=settings.BAutoClutch,auto_shift=settings.BAutoShift,force_manual=settings.BForceManual,
            force_automatic_transmission=car.forceAutomaticTransmission,h_shifter=settings.CBHShifter,
            analog_gas=settings.CBenableGas,analog_brake=settings.CBenableBrake,analog_clutch=settings.CBenableClutch,
            analog_steer=settings.CBenableSteer,analog_handbrake=settings.CBenableHandBrake,mouse_drive=inputscript.i.mouseDrive };
    }
    static int Gears(bool r,bool n,bool g1,bool g2,bool g3,bool g4,bool g5,bool g6)
    { return (r?1:0)|(n?2:0)|(g1?4:0)|(g2?8:0)|(g3?16:0)|(g4?32:0)|(g5?64:0)|(g6?128:0); }
    public static State Read(carscript car,seatscript seat,int revision)
    {
        var input=inputscript.i;
        return new State { input_revision=revision,mode=ReadMode(car),
            input_handbrake_press=input.handbrakeBDown,car_handbrake_press=car.ihandbrakeBDown,stored_handbrake=car.BhandBrake,
            input_ignition_held=input.ignition,input_ignition_down=input.ignitionDown,input_ignition_up=input.ignitionUp,
            car_ignition_held=car.iignition,car_ignition_down=car.iignitionDown,
            seat_driver0=seat.driverSeat0,seat_driver1=seat.driverSeat1,driving=car.driving,driving2=car.driving2,
            player_driving=car.isPlayerDriving,player_driving2=car.isPlayerDriving2,
            input_throttle=input.throttle,input_brake=input.brake,input_clutch=input.clutch,input_steer=input.steer,input_handbrake=input.handbrake,
            car_throttle=car.ithrottle,car_brake=car.ibrake,car_clutch=car.iclutch,car_steer=car.isteer,car_handbrake=car.ihandbrake,
            input_gears=Gears(input.grDown,input.gnDown,input.g1Down,input.g2Down,input.g3Down,input.g4Down,input.g5Down,input.g6Down),
            car_gears=Gears(car.igrDown,car.ignDown,car.ig1Down,car.ig2Down,car.ig3Down,car.ig4Down,car.ig5Down,car.ig6Down),
            selected_ratio=car.selectedgr,differential_ratio=car.differentialRatio,brake_power=car.brakePower,max_handbrake=car.maxHandBrake };
    }
}
