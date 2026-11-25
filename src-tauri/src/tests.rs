use crate::{
    models::*,
    window_controller::{align_switch, clamp},
};

#[test]
fn geometry_handles_negative_and_removed_monitors() {
    let work = Bounds {
        x: -1920.,
        y: 30.,
        width: 1920.,
        height: 1050.,
    };
    let b = clamp(
        Bounds {
            x: -2400.,
            y: -100.,
            width: 380.,
            height: 520.,
        },
        &work,
        &Presentation::Panel,
    );
    assert_eq!(b.x, -1920.);
    assert_eq!(b.y, 30.);
    let work = Bounds {
        x: 0.,
        y: 25.,
        width: 1280.,
        height: 775.,
    };
    let b = clamp(b, &work, &Presentation::Main);
    assert!(b.x >= 0. && b.y >= 25. && b.x + b.width <= 1280. && b.y + b.height <= 800.);
}

#[test]
fn return_buttons_stay_at_the_clicked_screen_position() {
    let cases = [
        (Presentation::Orb, Presentation::Panel, 23., 23.),
        (Presentation::Panel, Presentation::Orb, 23., 23.),
        (Presentation::Main, Presentation::Panel, 57., 57.),
        (Presentation::Panel, Presentation::Main, 57., 57.),
        (Presentation::Main, Presentation::Orb, 23., 57.),
        (Presentation::Orb, Presentation::Main, 57., 23.),
    ];
    let size = |mode: &Presentation| match mode {
        Presentation::Orb => (192., 36.),
        Presentation::Panel => (380., 520.),
        Presentation::Main => (1040., 720.),
    };
    for (source_mode, target, source_inset, target_inset) in cases {
        let (width, height) = size(&source_mode);
        let mut source = Bounds {
            x: -460.5,
            y: 148.5,
            width,
            height,
        };
        let (width, height) = size(&target);
        let destination = Bounds {
            x: 700.,
            y: 800.,
            width,
            height,
        };
        let anchor_x = source.x + source.width - source_inset;
        let anchor_y = source.y + 18.;
        for _ in 0..50 {
            let next = align_switch(destination.clone(), &source, &source_mode, &target);
            assert_eq!(next.x + next.width - target_inset, anchor_x);
            assert_eq!(next.y + 18., anchor_y);
            let restored = align_switch(source.clone(), &next, &target, &source_mode);
            assert_eq!(restored.x, source.x);
            assert_eq!(restored.y, source.y);
            source = restored;
        }
    }
}

#[test]
fn legacy_orb_bounds_become_a_fixed_mini_bar() {
    let work = Bounds {
        x: 0.,
        y: 25.,
        width: 1280.,
        height: 775.,
    };
    let bounds = clamp(
        Bounds {
            x: 1200.,
            y: 400.,
            width: 56.,
            height: 56.,
        },
        &work,
        &Presentation::Orb,
    );
    assert_eq!(bounds.width, 192.);
    assert_eq!(bounds.height, 36.);
    assert_eq!(bounds.x, 1088.);
    assert_eq!(bounds.y, 400.);
}
