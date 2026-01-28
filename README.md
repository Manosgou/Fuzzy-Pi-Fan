## Fuzzy Pi Fan

**Short story!**

I recently started using a Raspberry Pi 4 as my local file server. It didn't take many days to realize that it was suffering from high temperatures, which were leading to throttling. I immediately had to buy a fan and without much research I bought the official raspberry pi fan. Without further ado the rasbian os does not offer enough control on the fan operation. So I decided to make an app which will control the fan speed through fuzzy logic.

**Raspberry Pi PWM**

Hardware PWM must be enabled in order for this app to work.Activate it by adding an overlay in the /boot/config.txt. Main Raspberry Pi kernel documentation gives 2 possibilities. Either a [single channel](https://github.com/raspberrypi/linux/blob/04c8e47067d4873c584395e5cb260b4f170a99ea/arch/arm/boot/dts/overlays/README#L925), either a [dual channel](https://github.com/raspberrypi/linux/blob/04c8e47067d4873c584395e5cb260b4f170a99ea/arch/arm/boot/dts/overlays/README#L944). For our purpose we will activate only one PWM channel, which exposes the following GPIO pins

| PWM  | GPIO | Function | Alt  | dtoverlay                   |
| ---- | ---- | -------- | ---- | --------------------------- |
| PWM0 | 12   | 4        | Alt0 | dtoverlay=pwm,pin=12,func=4 |
| PWM0 | 18   | 2        | Alt5 | dtoverlay=pwm,pin=18,func=2 |
| PWM1 | 13   | 4        | Alt0 | dtoverlay=pwm,pin=13,func=4 |
| PWM1 | 19   | 2        | Alt5 | dtoverlay=pwm,pin=19,func=2 |

Edit the **/boot/firmware/config.txt** _(OLD: /boot/config.txt)_ file and add the dtoverlay line in the file. _You need root privileges for this_:

```bash
sudo nano /boot/firmware/config.txt
```

Save the file and reboot:

```bash
sudo reboot
```

After rebooting your Pi, you will have access to hardware PWM. A new sysfs directory will be shown uder the following route /sys/class/pwm/pwmchip{num}/pwm{num}, which operates much like the sysfs support for GPIO.

The numbers in brackets (_pwmchip{num} , pwm{num}_) correspond to values of the variables:

```rust
const  BB_PWM_CHIP:  u32  =  0;
const  BB_PWM_NUMBER:  u32  =  0;
```

If pwm{num} is missing, do the following (replace {num} with the pwm number according to your configuration): 

```bash
echo {num} > export
```

**Prerequisites**

The app is developed using the Rust programming language and uses the following dependencies:

```toml
[dependencies]
sysfs-pwm = { git = "https://github.com/rust-embedded/rust-sysfs-pwm", branch = "master" }
rsfuzzy={git="https://github.com/auseckas/rsfuzzy",branch = "master"}
cpu-monitor = "0.1.1"
```

_(Huge thanks to the creators ❤️)_

**Fuzzy Logic**

The basic idea of this application is to use a fuzzy logic model in order to control the speed of the fan. The temperature of the pi processor is entered as a parameter in the model which then outputs the fan speed. The following plot shows the membership functions:

![plot](./images/mf_plot.png)

You can easily edit the membership functions, by updating the following code section:

```rust
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
```

The fuzzy relationship between input (soc_temp,cpu_usage) and output (fan_speed) is defined in the following rules:

```rust
let f_rules = vec![
        "if soc_temp is cold or cpu_usage is low then fan_speed is low",
        "if soc_temp is warm or cpu_usage is medium then fan_speed is moderate",
        "if soc_temp is hot or cpu_usage is high then fan_speed is high"
    ];
```

Finally we use **Centroid** method for the defuzzification:

```rust
f_engine.add_defuzz("centroid");
```

**Run & Build**

Use cargo in order to run and build the project. **Be sure you have installed the appropriate rust compiler (toolchain)**
