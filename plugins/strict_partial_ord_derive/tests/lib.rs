#[cfg(test)]
use std::cmp::Ordering;
use std::cmp::Ordering::*;
use std::f32::NAN;

#[derive(Debug, PartialEq, strict_partial_ord_derive::PartialOrd)]
struct Point3 {
    x: f32,
    y: f32,
    z: f32,
}

impl From<(f32, f32, f32)> for Point3 {
    fn from((x, y, z): (f32, f32, f32)) -> Self {
        Self { x, y, z }
    }
}

// Point3 PartialCmp Test function
fn p3_pcmp_test<T: Into<Point3>>(p1: T, p2: T, expected: Option<Ordering>) {
    assert_eq!(p1.into().partial_cmp(&p2.into()), expected);
}

#[test]
fn basic_equality() {
    p3_pcmp_test((0., 0., 0.), (0., 0., 0.), Some(Equal));
}

#[test]
fn lesser() {
    p3_pcmp_test((0., 0., 0.), (1., 1., 1.), Some(Less));
    p3_pcmp_test((0., 1., 2.), (1., 2., 3.), Some(Less));
}

#[test]
fn greater() {
    p3_pcmp_test((1., 1., 1.), (0., 0., 0.), Some(Greater));
    p3_pcmp_test((1., 2., 3.), (0., 1., 2.), Some(Greater));
}

#[test]
fn not_comparable() {
    p3_pcmp_test((0., 0., 0.), (-1., 0., 1.), None);
    p3_pcmp_test((-1., 0., 1.), (0., 0., 0.), None);
    p3_pcmp_test((0., 0., 0.), (0., NAN, 0.), None);
}
