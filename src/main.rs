use cpu_monitor::CpuInstant;
use rsfuzzy::Engine;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::thread;
use std::time::Duration;
use sysfs_pwm::Pwm;

const BB_PWM_CHIP: u32 = 0;
const BB_PWM_NUMBER: u32 = 0;

fn thermals() -> Option<f32> {
    fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
        .ok()
        .and_then(|temp_string: String| {
            temp_string.trim().parse().ok().and_then(|temp_float: f32| {
                Some(f32::trunc((temp_float / 1_000_f32) * 10.0) / 10.0)
            })
        })
}

fn cpu_usage(start: CpuInstant) -> Option<f32> {
    let end = CpuInstant::now().ok();
    if end.is_some() {
        let duration = end.unwrap() - start;
        let cpu_percentage = (duration.non_idle() * 100.0) as f32;
        return Some(f32::trunc(cpu_percentage * 10.0) / 10.0);
    }
    None
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut f_engine = Engine::new();
    let pwm = Pwm::new(BB_PWM_CHIP, BB_PWM_NUMBER)?;

    let soc_temp = rsfuzzy::fz_input_var![
        ("down", "cold", vec![30.0, 60.0]),
        ("triangle", "warm", vec![40.0, 60.0, 70.0]),
        ("up", "hot", vec![60.0, 90.0])
    ];

    f_engine.add_input_var("soc_temp", soc_temp, 30, 90);

    let fan_speed = rsfuzzy::fz_output_var![
        ("down", "low", vec![0.0, 40.0]),
        ("triangle", "moderate", vec![20.0, 60.0, 70.0]),
        ("up", "high", vec![60.0, 100.0])
    ];
    f_engine.add_output_var("fan_speed", fan_speed, 0, 100);

    let f_rules = vec![
        "if soc_temp is cold then fan_speed is low",
        "if soc_temp is warm then fan_speed is moderate",
        "if soc_temp is hot then fan_speed is high",
    ];

    f_engine.add_rules(f_rules);
    f_engine.add_defuzz("centroid");

    pwm.with_exported(|| {
        pwm.enable(true)?;
        pwm.set_period_ns(20_000_000)?;
        loop {
            let start = CpuInstant::now()?;
            thread::sleep(Duration::from_secs(10));
            let period_ns = pwm.get_period_ns()?;
            let soc_temp = thermals();
            if soc_temp.is_none() {
                panic!("SOC temperature can't be fetched")
            }
            let cpu_percentage = cpu_usage(start);
            if cpu_percentage.is_none() {
                panic!("CPU usage can't be fetched")
            }
            let inputs = rsfuzzy::fz_set_inputs![
                ("soc_temp", soc_temp.unwrap()),
                ("cpu_usage", cpu_percentage.unwrap())
            ];
            let result = f_engine.calculate(inputs);
            let duty_cycle = f32::trunc(result * 10.0) / 1000.0;
            pwm.set_duty_cycle_ns(((period_ns as f32) * duty_cycle) as u32)?;
        }
    })?;
    Ok(())
}
