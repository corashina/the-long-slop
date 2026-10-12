//! Per-layer installed System.Random1234 sequence for Dithering.Render.
//! Calls follow that layer's render operations, never elapsed wall time.
#[derive(Clone)]
pub struct DitherSequence {
    seeds: [i32; 56],
    next: usize,
    next_pair: usize,
    noise_index: usize,
}
impl Default for DitherSequence {
    fn default() -> Self {
        let mut seeds = [0_i32; 56];
        let mut mj = 161803398 - 1234;
        let mut mk = 1;
        let mut ii = 0;
        seeds[55] = mj;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55
            }
            seeds[ii] = mk;
            mk = mj - mk;
            if mk < 0 {
                mk += i32::MAX
            }
            mj = seeds[ii];
        }
        for _ in 0..4 {
            for i in 1..56 {
                let mut j = i + 30;
                if j >= 55 {
                    j -= 55
                }
                seeds[i] -= seeds[1 + j];
                if seeds[i] < 0 {
                    seeds[i] += i32::MAX
                }
            }
        }
        Self {
            seeds,
            next: 0,
            next_pair: 21,
            noise_index: 0,
        }
    }
}
impl DitherSequence {
    fn sample(&mut self) -> f32 {
        let mut a = self.next + 1;
        if a >= 56 {
            a = 1
        }
        let mut b = self.next_pair + 1;
        if b >= 56 {
            b = 1
        }
        let mut value = self.seeds[a] - self.seeds[b];
        if value == i32::MAX {
            value -= 1
        }
        if value < 0 {
            value += i32::MAX
        }
        self.seeds[a] = value;
        self.next = a;
        self.next_pair = b;
        // Exact installed double literal; source then converts each NextDouble to float.
        (f64::from(value) * f64::from_bits(0x3e00000000200000)) as f32
    }
    pub fn render(&mut self, extent: [u32; 2]) -> (usize, [f32; 4]) {
        self.noise_index += 1;
        if self.noise_index >= 64 {
            self.noise_index = 0
        }
        (
            self.noise_index,
            [
                extent[0] as f32 / 64.,
                extent[1] as f32 / 64.,
                self.sample(),
                self.sample(),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_sequence_and_noise_wrap_preserve_each_layer_state() {
        let mut layer = DitherSequence::default();
        let expected = [
            [0x3ecc5458, 0x3f6559ab],
            [0x3ea36e91, 0x3f725d64],
            [0x3eadca8f, 0x3f72e321],
            [0x3f4ed88e, 0x3f054ea0],
            [0x3f24da70, 0x3ea007ba],
            [0x3ed5b28a, 0x3f5e72db],
            [0x3f728001, 0x3eedd4cc],
            [0x3f3f57ce, 0x3f306c55],
            [0x3f15e0e2, 0x3d989c32],
            [0x3da8037d, 0x3e006d3b],
            [0x3f477683, 0x3f17f203],
            [0x3f0b4927, 0x3e00736b],
            [0x3f239502, 0x3e5b4207],
            [0x3f624cc6, 0x3e9153cf],
            [0x3ec98cd2, 0x3f65148e],
            [0x3f40a112, 0x3efea418],
        ];
        for (i, bits) in expected.iter().enumerate() {
            let (index, coords) = layer.render([1280, 720]);
            assert_eq!(index, i + 1);
            assert_eq!(coords[..2], [20., 11.25]);
            assert_eq!([coords[2].to_bits(), coords[3].to_bits()], *bits);
        }
        assert_eq!(
            layer.seeds,
            [
                0, 857019877, 1923929452, 685483091, 2033103372, 728933312, 2037485757, 1735149371,
                1118261193, 1382889412, 671215230, 896311924, 1866034552, 2034237595, 997536529,
                1605101337, 1479944834, 1257271596, 160023322, 176175054, 269330267, 1673216339,
                1274610059, 1168413542, 269380965, 1372225755, 459817170, 1898341129, 609547231,
                845362304, 1921664754, 1615890720, 1068041704, 1139327282, 2111326198, 1993294189,
                296300284, 1143837930, 638411224, 15963364, 1235302158, 1395688165, 153156519,
                1700682294, 494753033, 2005045501, 801842440, 252442491, 544252186, 962707885,
                16045252, 774971950, 2056438780, 1118105565, 26616244, 123415219
            ]
        );
        assert_eq!((layer.next, layer.next_pair), (32, 53));
        for i in 17..=65 {
            assert_eq!(layer.render([1279, 719]).0, i % 64);
        }
        let mut other = DitherSequence::default();
        let (index, coords) = other.render([1280, 720]);
        assert_eq!(index, 1);
        assert_eq!([coords[2].to_bits(), coords[3].to_bits()], expected[0]);
    }
}
