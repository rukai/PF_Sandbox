extern crate pf_sandbox_lib;
use pf_sandbox_lib::input::stick_deadzone;

#[test]
fn stick_deadzone_test() {
    // stick_deadzone(*, 0)
    assert_eq!(stick_deadzone(0, 0), 128);
    assert_eq!(stick_deadzone(1, 0), 129);
    assert_eq!(stick_deadzone(126, 0), 254);
    assert_eq!(stick_deadzone(127, 0), 255);
    assert_eq!(stick_deadzone(255, 0), 255);

    // stick_deadzone(*, 127)
    assert_eq!(stick_deadzone(0, 127), 1);
    assert_eq!(stick_deadzone(1, 127), 2);
    assert_eq!(stick_deadzone(127, 127), 128);
    assert_eq!(stick_deadzone(128, 127), 129);
    assert_eq!(stick_deadzone(129, 127), 130);
    assert_eq!(stick_deadzone(253, 127), 254);
    assert_eq!(stick_deadzone(254, 127), 255);
    assert_eq!(stick_deadzone(255, 127), 255);

    // stick_deadzone(*, 128)
    assert_eq!(stick_deadzone(0, 128), 0);
    assert_eq!(stick_deadzone(1, 128), 1);
    assert_eq!(stick_deadzone(127, 128), 127);
    assert_eq!(stick_deadzone(128, 128), 128);
    assert_eq!(stick_deadzone(129, 128), 129);
    assert_eq!(stick_deadzone(254, 128), 254);
    assert_eq!(stick_deadzone(255, 128), 255);

    // stick_deadzone(*, 129)
    assert_eq!(stick_deadzone(0, 129), 0);
    assert_eq!(stick_deadzone(1, 129), 0);
    assert_eq!(stick_deadzone(2, 129), 1);
    assert_eq!(stick_deadzone(127, 129), 126);
    assert_eq!(stick_deadzone(128, 129), 127);
    assert_eq!(stick_deadzone(129, 129), 128);
    assert_eq!(stick_deadzone(254, 129), 253);
    assert_eq!(stick_deadzone(255, 129), 254);

    // stick_deadzone(*, 255)
    assert_eq!(stick_deadzone(0, 255), 0);
    assert_eq!(stick_deadzone(127, 255), 0);
    assert_eq!(stick_deadzone(128, 255), 1);
    assert_eq!(stick_deadzone(129, 255), 2);
    assert_eq!(stick_deadzone(254, 255), 127);
    assert_eq!(stick_deadzone(255, 255), 128);
}
