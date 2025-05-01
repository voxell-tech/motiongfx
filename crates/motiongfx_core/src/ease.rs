use std::f32::consts::PI;

pub type EaseFn = fn(t: f32) -> f32;

pub fn linear(t: f32) -> f32 {
    t
}

// Easing taken from https://easings.net/

const C1: f32 = 1.70158;
const C2: f32 = C1 * 1.525;
const C3: f32 = C1 + 1.0;
const C4: f32 = PI * 2.0 / 3.0;
const C5: f32 = PI * 2.0 / 4.5;

pub mod sine {
    use std::f32::consts::PI;

    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        1.0 - f32::cos(t * PI * 0.5)
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        f32::cos(t * PI * 0.5)
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        -(f32::cos(PI * t) - 1.0) * 0.5
    }
}

pub mod quad {
    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        t * t
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        let t = 1.0 - t;
        1.0 - t * t
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            let t = 1.0 - t;
            1.0 - t * t * 2.0
        }
    }
}

pub mod cubic {
    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        t * t * t
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        let t = 1.0 - t;
        1.0 - t * t * t
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = 1.0 - t;
            1.0 - t * t * t * 4.0
        }
    }
}

pub mod quart {
    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        t * t * t * t
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        let t = 1.0 - t;
        1.0 - t * t * t * t
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            let t = 1.0 - t;
            1.0 - t * t * t * t * 8.0
        }
    }
}

pub mod quint {
    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        t * t * t * t * t
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        let t = 1.0 - t;
        1.0 - t * t * t * t * t
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            let t = 1.0 - t;
            1.0 - t * t * t * t * t * 16.0
        }
    }
}

pub mod expo {
    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else {
            f32::powf(2.0, 10.0 * t - 10.0)
        }
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - f32::powf(2.0, -10.0 * t)
        }
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            f32::powf(2.0, 20.0 * t - 10.0) * 0.5
        } else {
            (2.0 - f32::powf(2.0, -20.0 * t + 10.0)) * 0.5
        }
    }
}

pub mod circ {
    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        1.0 - f32::sqrt(1.0 - t * t)
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        let t = t - 1.0;
        f32::sqrt(1.0 - t * t)
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            let t = 2.0 * t;
            (1.0 - f32::sqrt(1.0 - t * t)) * 0.5
        } else {
            let t = 1.0 - t;
            (f32::sqrt(1.0 - t * t * 4.0) + 1.0) * 0.5
        }
    }
}

pub mod back {
    use super::*;

    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        C3 * t * t * t - C1 * t * t
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        let t = t - 1.0;
        1.0 + C3 * t * t * t + C1 * t * t
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            let t = 2.0 * t;
            (t * t * ((C2 + 1.0) * t - C2)) * 0.5
        } else {
            let t = 2.0 * t - 2.0;
            (t * t * ((C2 + 1.0) * t + C2) + 2.0) * 0.5
        }
    }
}

pub mod elastic {
    use super::*;

    #[inline]
    pub fn ease_in(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let t = 10.0 * t;
            -f32::powf(2.0, t - 10.0) * f32::sin((t - 10.75) * C4)
        }
    }

    #[inline]
    pub fn ease_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let t = 10.0 * t;
            f32::powf(2.0, -t) * f32::sin((t - 0.75) * C4) + 1.0
        }
    }

    #[inline]
    pub fn ease_in_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            let t20 = 20.0 * t;
            if t < 0.5 {
                -f32::powf(2.0, t20 - 10.0)
                    * f32::sin((t20 - 11.125) * C5)
                    * 0.5
            } else {
                f32::powf(2.0, -t20 + 10.0)
                    * f32::sin((t20 - 11.125) * C5)
                    * 0.5
                    + 1.0
            }
        }
    }
}
