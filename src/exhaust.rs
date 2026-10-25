//! Source exhaust smoke rules: `fluidcontainer.GetBurnColor` and `carscript` smoke rate.
use crate::fluids::FluidContainer;
use crate::simulation::dials::inverse_lerp;

/// Source `GetBurnColor`: each fluid adds its color by share; each later fluid halves the sum.
pub fn burn_color(tank: &FluidContainer, colors: &[[f32; 4]; 6]) -> [f32; 4] {
    let total = tank.total();
    let mut color = [0.; 4];
    if !(total > 0. && total.is_finite()) {
        return color;
    }
    for (i, entry) in tank.entries().enumerate() {
        let weight = inverse_lerp(0., 0.15, entry.amount / total);
        for (c, f) in color.iter_mut().zip(colors[entry.fluid_type as usize]) {
            *c += f * weight;
            if i != 0 {
                *c *= 0.5;
            }
        }
    }
    color
}

/// Source smoke emission rate: scales with rpm while the engine runs.
pub fn smoke_rate(running: bool, rpm: f32, max_rpm: f32, rate: f32) -> f32 {
    if running {
        inverse_lerp(0., max_rpm, rpm) * rate
    } else {
        0.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::FluidEntry;

    const COLORS: [[f32; 4]; 6] = [
        [1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., 0., 1., 1.],
        [0.5, 0.5, 0.5, 1.],
        [0.2, 0.4, 0.6, 0.8],
        [0., 0., 0., 0.],
    ];

    fn tank(entries: &[(u8, f32)]) -> FluidContainer {
        let entries: Vec<_> = entries
            .iter()
            .map(|&(fluid_type, amount)| FluidEntry { fluid_type, amount })
            .collect();
        FluidContainer::new(100., &entries)
    }

    fn close(a: [f32; 4], b: [f32; 4]) {
        for i in 0..4 {
            assert!((a[i] - b[i]).abs() < 1e-5, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn single_fluid_gives_its_color() {
        close(burn_color(&tank(&[(3, 10.)]), &COLORS), COLORS[3]);
    }

    #[test]
    fn later_fluids_halve_the_sum_in_stored_order() {
        // Shares 0.9 and 0.1: weights 1 and 2/3.
        let w = 0.1 / 0.15;
        let c = burn_color(&tank(&[(0, 9.), (1, 1.)]), &COLORS);
        close(c, [0.5, 0.5 * w, 0., 0.5 * (1. + w)]);
        // Three fluids at 50/25/25: all weights 1.
        let c = burn_color(&tank(&[(0, 2.), (1, 1.), (2, 1.)]), &COLORS);
        close(c, [0.25, 0.25, 0.5, 0.25 + 0.25 + 0.5]);
    }

    #[test]
    fn weight_ramps_to_fifteen_percent() {
        let c = burn_color(&tank(&[(2, 1.), (0, 0.)]), &COLORS);
        close(c, [0., 0., 0.5, 0.5]);
        let c = burn_color(&tank(&[(2, 92.5), (0, 7.5)]), &COLORS);
        close(c, [0.25, 0., 0.5, 0.75]);
        let c = burn_color(&tank(&[(2, 85.), (0, 15.)]), &COLORS);
        close(c, [0.5, 0., 0.5, 1.]);
    }

    #[test]
    fn empty_tank_gives_no_color() {
        assert_eq!(burn_color(&tank(&[]), &COLORS), [0.; 4]);
        assert_eq!(burn_color(&tank(&[(0, 0.)]), &COLORS), [0.; 4]);
    }

    #[test]
    fn smoke_rate_follows_rpm_while_running() {
        assert_eq!(smoke_rate(false, 3000., 3000., 20.), 0.);
        assert_eq!(smoke_rate(true, 3000., 3000., 20.), 20.);
        assert_eq!(smoke_rate(true, 1500., 3000., 20.), 10.);
        assert_eq!(smoke_rate(true, 6000., 3000., 20.), 20.);
    }
}
