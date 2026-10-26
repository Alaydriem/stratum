use stratum_core::services::DemoClock;

#[test]
fn demo_steps_every_half_second_and_wraps() {
    let mut clock = DemoClock::new(0.5);

    let layers: Vec<i32> = [0.0, 0.49, 0.5, 1.0, 1.5].iter().map(|now| clock.layer(*now, 10, 12)).collect();

    assert_eq!(layers, [10, 10, 11, 12, 10]);
}

#[test]
fn demo_with_one_layer_stays_on_it() {
    let mut clock = DemoClock::new(0.5);

    for now in [0.0, 0.5, 1.0, 7.3] {
        assert_eq!(clock.layer(now, 5, 5), 5);
    }
}

#[test]
fn demo_starts_at_the_bottom_whenever_first_asked() {
    let mut clock = DemoClock::new(0.5);

    assert_eq!(clock.layer(100.0, 0, 4), 0);
    assert_eq!(clock.layer(100.5, 0, 4), 1);
}

#[test]
fn demo_without_a_step_stays_on_the_bottom() {
    let mut clock = DemoClock::new(0.0);

    for now in [5.0, 6.0, 50.0] {
        assert_eq!(clock.layer(now, 3, 6), 3);
    }
}
