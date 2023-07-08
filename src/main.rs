use std::fs;
use std::ops::Not;
use sysfs_pwm::{ Pwm };
use std::{ thread, time };
use rsfuzzy::{ Engine };
use std::collections::HashMap;
use cpu_monitor::CpuInstant;

const BB_PWM_CHIP: u32 = 0;
const BB_PWM_NUMBER: u32 = 0;

fn thermals() -> Option<f32> {
    let temp =
        fs
            ::read_to_string("/sys/class/thermal/thermal_zone0/temp")
            .unwrap()
            .trim()
            .parse::<f32>()
            .unwrap() / 1_000_f32;
    Some(f32::trunc(temp * 10.0) / 10.0)
}

fn main() {
    let mut f_engine = Engine::new();
    let pwm = Pwm::new(BB_PWM_CHIP, BB_PWM_NUMBER).unwrap();
    let start = CpuInstant::now().unwrap();

    let soc_temp = rsfuzzy::fz_input_var![
        ("down", "cold", vec![30.0, 60.0]),
        ("triangle", "warm", vec![40.0, 60.0, 80.0]),
        ("up", "hot", vec![60.0, 90.0])
    ];

    f_engine.add_input_var("soc_temp", soc_temp, 30, 90);

    let cpu_usage = rsfuzzy::fz_input_var![
        ("down", "low", vec![0.0, 50.0]),
        ("triangle", "medium", vec![25.0, 50.0, 75.0]),
        ("up", "high", vec![50.0, 100.0])
    ];

    f_engine.add_input_var("cpu_usage", cpu_usage, 0, 100);

    let fan_speed = rsfuzzy::fz_output_var![
        ("down", "low", vec![0.0, 50.0]),
        ("triangle", "moderate", vec![25.0, 50.0, 75.0]),
        ("up", "high", vec![50.0, 100.0])
    ];
    f_engine.add_output_var("fan_speed", fan_speed, 0, 100);

    let f_rules = vec![
        "if soc_temp is cold or cpu_usage is low then fan_speed is low",
        "if soc_temp is warm or cpu_usage is medium then fan_speed is moderate",
        "if soc_temp is hot or cpu_usage is high then fan_speed is high"
    ];

    f_engine.add_rules(f_rules);
    f_engine.add_defuzz("centroid");

    pwm.with_exported(|| {
        let is_enabled = pwm.get_enabled();
        if is_enabled?.not() {
            pwm.enable(true).unwrap();
        }
        pwm.set_period_ns(20_000_000).unwrap();
        loop {
            thread::sleep(time::Duration::from_secs(25));
            let period_ns = pwm.get_period_ns()?;
            let temp = thermals().unwrap();

            let end = CpuInstant::now().unwrap();
            let duration = end.clone() - start;
            let cpu_percentage = duration.non_idle() * 100.0;

            let inputs = rsfuzzy::fz_set_inputs![
                ("soc_temp", temp),
                ("cpu_usage", cpu_percentage as f32)
            ];
            let result = f_engine.calculate(inputs);

            let duty_cycle = f32::trunc(result * 10.0) / 1000.0;
            pwm.set_duty_cycle_ns(((period_ns as f32) * duty_cycle) as u32)?;
        }
    }).unwrap()
}
