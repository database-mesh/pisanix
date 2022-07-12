
#[macro_export]
macro_rules! execute_tc_command {
    ($attr:expr, $args:expr, $command_type:tt) => {
        {
            if let Some(netns) = &$attr.netns {
                $args.insert(0, "-n");
                $args.insert(1, netns);
            }

            let out = Command::new("tc")
                .args(&$args)
                .output()
                .expect("failed to add class");

            if !out.status.success() {
                println!("Failed to {} to device {}: {:?}", $command_type, $attr.device, out);
                return false;
            }

            true
        }
    }
}

