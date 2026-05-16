/// ADSR envelope tests
///
/// Covers all 5 envelope phases, rate-table tick gating, the attack
/// fast-path (rate=15), exponential decay/sustain steps, release
/// fixed-rate fade, and the full A→D→S→R→Off cycle.

use apu::dsp::{Adsr, EnvelopePhase};

// ============================================================
// ADSR — EnvelopePhase::Off
// ============================================================

#[test]
fn test_adsr_off_does_nothing() {
    let mut adsr = Adsr::default();
    // Default phase is Off; envelope_level must stay 0 forever.
    for _ in 0..1000 {
        adsr.update_envelope();
        assert_eq!(adsr.envelope_level, 0);
        assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
    }
}

// ============================================================
// ADSR — Attack
// ============================================================

#[test]
fn test_adsr_attack_rate15_jumps_1024_per_tick() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15; // fast-path: no rate gating

    adsr.update_envelope();
    // Should jump straight by 1024 (or hit 0x7FF if it was near the top)
    assert!(adsr.envelope_level >= 1024 || adsr.envelope_level == 0x7FF);
}

#[test]
fn test_adsr_attack_rate15_reaches_max_within_2_ticks() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15;
    adsr.update_envelope(); // +1024 → 1024
    adsr.update_envelope(); // could hit max
    // After at most ceil(0x7FF / 1024) = 2 ticks we must be at max or in Decay
    assert!(
        adsr.envelope_level == 0x7FF || adsr.envelope_phase == EnvelopePhase::Decay,
        "level={:#05X} phase={:?}", adsr.envelope_level, adsr.envelope_phase
    );
}

#[test]
fn test_adsr_attack_normal_rate_gated() {
    // attack_rate=0 → rate_idx=1 → period=2048 ticks between steps.
    // After 1 tick nothing should have changed.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 0;

    adsr.update_envelope(); // first tick: counter=1, not yet due
    assert_eq!(adsr.envelope_level, 0, "should not step yet");
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Attack);
}

#[test]
fn test_adsr_attack_transitions_to_decay_at_max() {
    // Use rate=15 to reach max quickly.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15;

    let mut reached_decay = false;
    for _ in 0..10 {
        adsr.update_envelope();
        if adsr.envelope_phase == EnvelopePhase::Decay {
            reached_decay = true;
            break;
        }
    }
    assert!(reached_decay, "Attack must transition to Decay on hitting 0x7FF");
    assert_eq!(adsr.envelope_level, 0x7FF, "level must be exactly 0x7FF on Decay entry");
}

#[test]
fn test_adsr_attack_level_never_exceeds_max() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate = 15;
    for _ in 0..20 {
        adsr.update_envelope();
        assert!(adsr.envelope_level <= 0x7FF, "level={:#05X}", adsr.envelope_level);
    }
}

// ============================================================
// ADSR — Decay
// ============================================================

#[test]
fn test_adsr_decay_falls_toward_sustain_target() {
    // decay_rate=7 → rate_idx = 7*2+16 = 30 → period=2 (very fast)
    let mut adsr = Adsr::default();
    adsr.envelope_phase  = EnvelopePhase::Decay;
    adsr.decay_rate      = 7;
    adsr.sustain_level   = 3; // target = (3+1)*0x100 = 0x400
    adsr.envelope_level  = 0x7FF;

    let mut hit_sustain = false;
    for _ in 0..5000 {
        adsr.update_envelope();
        if adsr.envelope_phase == EnvelopePhase::Sustain {
            hit_sustain = true;
            break;
        }
    }
    assert!(hit_sustain, "Decay must eventually reach Sustain");
    let expected_target = (adsr.sustain_level as u16 + 1) * 0x100;
    assert_eq!(adsr.envelope_level, expected_target, "must land exactly on sustain target");
}

#[test]
fn test_adsr_decay_step_is_exponential() {
    // At high levels the step is larger than at low levels.
    // decay_rate=7 (period=2), run two steps from two different starting points.
    let step_at = |start: u16| -> u16 {
        let mut adsr = Adsr::default();
        adsr.envelope_phase = EnvelopePhase::Decay;
        adsr.decay_rate     = 7;
        adsr.sustain_level  = 0; // target = 0x100
        adsr.envelope_level = start;
        let before = adsr.envelope_level;
        // Pump until at least one step fires
        for _ in 0..10 {
            let pre = adsr.envelope_level;
            adsr.update_envelope();
            if adsr.envelope_level != pre || adsr.envelope_phase != EnvelopePhase::Decay {
                break;
            }
        }
        before.saturating_sub(adsr.envelope_level)
    };

    let step_high = step_at(0x700);
    let step_low  = step_at(0x200);
    assert!(step_high > step_low, "exponential: high={step_high} low={step_low}");
}

#[test]
fn test_adsr_decay_rate0_is_slow() {
    // decay_rate=0 → rate_idx=16 → period=64: after 10 ticks, no step.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Decay;
    adsr.decay_rate     = 0;
    adsr.sustain_level  = 0;
    adsr.envelope_level = 0x7FF;

    for _ in 0..10 {
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_level, 0x7FF, "decay_rate=0 should not step within 10 ticks");
}

// ============================================================
// ADSR — Sustain
// ============================================================

#[test]
fn test_adsr_sustain_rate0_holds_forever() {
    // sustain_rate=0 → period=0 → tick_due always returns false → level never changes.
    let mut adsr = Adsr::default();
    adsr.envelope_phase  = EnvelopePhase::Sustain;
    adsr.sustain_rate    = 0;
    adsr.envelope_level  = 0x400;

    for _ in 0..10_000 {
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_level, 0x400, "sustain_rate=0 must hold level indefinitely");
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Sustain);
}

#[test]
fn test_adsr_sustain_decreases_with_nonzero_rate() {
    // sustain_rate=31 → period=1 (every tick)
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Sustain;
    adsr.sustain_rate   = 31;
    adsr.envelope_level = 0x400;
    let before = adsr.envelope_level;

    adsr.update_envelope();
    assert!(adsr.envelope_level < before, "level must decrease with sustain_rate=31");
}

#[test]
fn test_adsr_sustain_reaches_off_at_zero() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Sustain;
    adsr.sustain_rate   = 31;
    adsr.envelope_level = 1; // one step away from 0

    // The step formula is (level >> 8) + 1 = (0 >> 8) + 1 = 1, so one tick should silence it.
    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 0);
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
}

#[test]
fn test_adsr_sustain_step_is_exponential() {
    // Higher level → bigger step, like Decay.
    let step_at = |start: u16| -> u16 {
        let mut adsr = Adsr::default();
        adsr.envelope_phase = EnvelopePhase::Sustain;
        adsr.sustain_rate   = 31;
        adsr.envelope_level = start;
        let before = adsr.envelope_level;
        adsr.update_envelope();
        before.saturating_sub(adsr.envelope_level)
    };
    let step_high = step_at(0x700);
    let step_low  = step_at(0x100);
    assert!(step_high > step_low, "sustain exponential: high={step_high} low={step_low}");
}

#[test]
fn test_tick_due_period_zero_never_fires() {
    // period=0 (sustain_rate=0) must never step the envelope — covers the
    // early-return guard inside tick_due.
    let mut adsr = Adsr::default();
    adsr.envelope_phase  = EnvelopePhase::Sustain;
    adsr.sustain_rate    = 0; // ENVELOPE_RATE_TABLE[0] = 0
    adsr.envelope_level  = 0x400;

    for _ in 0..100_000 {
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_level, 0x400, "period=0 must never step");
}

#[test]
fn test_tick_due_fires_exactly_at_period() {
    // decay_rate=7 → period = ENVELOPE_RATE_TABLE[30] = 2.
    // Must not step on tick 1, must step on tick 2.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Decay;
    adsr.decay_rate     = 7;
    adsr.sustain_level  = 0;
    adsr.envelope_level = 0x7FF;

    let before = adsr.envelope_level;
    adsr.update_envelope(); // tick 1
    assert_eq!(adsr.envelope_level, before, "must not step on first tick");
    adsr.update_envelope(); // tick 2
    assert!(adsr.envelope_level < before, "must step on second tick (period=2)");
}

// ============================================================
// ADSR — Release
// ============================================================

#[test]
fn test_adsr_release_decreases_by_8_per_tick() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Release;
    adsr.envelope_level = 100;

    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 92, "release must subtract exactly 8");
}

#[test]
fn test_adsr_release_reaches_off() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Release;
    adsr.envelope_level = 0x7FF;

    for _ in 0..300 {
        adsr.update_envelope();
        if adsr.envelope_phase == EnvelopePhase::Off { break; }
    }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
    assert_eq!(adsr.envelope_level, 0);
}

#[test]
fn test_adsr_release_clamps_at_zero_not_underflow() {
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Release;
    adsr.envelope_level = 4; // 4 - 8 would underflow without saturating_sub

    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 0);
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
}

// ============================================================
// ADSR — Full A→D→S→R→Off cycle
// ============================================================

#[test]
fn test_adsr_full_cycle() {
    let mut adsr = Adsr::default();
    adsr.attack_rate    = 15;  // instant
    adsr.decay_rate     = 7;   // fast
    adsr.sustain_level  = 2;   // target = 0x300
    adsr.sustain_rate   = 31;  // fast sustain drain
    adsr.envelope_phase = EnvelopePhase::Attack;

    // Attack → Decay
    while adsr.envelope_phase == EnvelopePhase::Attack { adsr.update_envelope(); }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Decay);

    // Decay → Sustain
    for _ in 0..10_000 {
        if adsr.envelope_phase != EnvelopePhase::Decay { break; }
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Sustain);
    let target = (adsr.sustain_level as u16 + 1) * 0x100;
    assert_eq!(adsr.envelope_level, target);

    // Sustain → Off
    for _ in 0..10_000 {
        if adsr.envelope_phase == EnvelopePhase::Off { break; }
        adsr.update_envelope();
    }
    assert_eq!(adsr.envelope_phase, EnvelopePhase::Off);
    assert_eq!(adsr.envelope_level, 0);
}

#[test]
fn test_adsr_key_off_mid_attack_enters_release() {
    // Even if still in Attack, switching phase to Release should work normally.
    let mut adsr = Adsr::default();
    adsr.envelope_phase = EnvelopePhase::Attack;
    adsr.attack_rate    = 0; // slow
    adsr.envelope_level = 500;

    // Simulate key-off: caller sets phase to Release
    adsr.envelope_phase = EnvelopePhase::Release;

    adsr.update_envelope();
    assert_eq!(adsr.envelope_level, 492, "release from mid-attack: 500 - 8 = 492");
}
