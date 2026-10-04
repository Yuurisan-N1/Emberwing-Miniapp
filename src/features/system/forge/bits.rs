#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct Bits {
    w: Vec<u64>,
}

impl Bits {
    pub fn zeros(n: usize) -> Self {
        Bits {
            w: vec![0u64; (n + 63) / 64],
        }
    }

    pub fn ones(n: usize) -> Self {
        let mut b = Bits::zeros(n);
        for i in 0..n {
            b.set(i);
        }
        b
    }


    pub fn set(&mut self, i: usize) {
        if i >> 6 < self.w.len() {
            self.w[i >> 6] |= 1u64 << (i & 63);
        }
    }

    pub fn clear(&mut self, i: usize) {
        if i >> 6 < self.w.len() {
            self.w[i >> 6] &= !(1u64 << (i & 63));
        }
    }

    pub fn test(&self, i: usize) -> bool {
        i >> 6 < self.w.len() && (self.w[i >> 6] >> (i & 63)) & 1 == 1
    }

    pub fn is_empty(&self) -> bool {
        self.w.iter().all(|&x| x == 0)
    }

    pub fn count(&self) -> usize {
        self.w.iter().map(|x| x.count_ones() as usize).sum()
    }

    pub fn intersects(&self, o: &Bits) -> bool {
        self.w.iter().zip(o.w.iter()).any(|(a, b)| a & b != 0)
    }



    pub fn words(&self) -> &[u64] {
        &self.w
    }

    pub fn ids(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for (k, &x) in self.w.iter().enumerate() {
            let mut x = x;
            while x != 0 {
                let b = x.trailing_zeros() as usize;
                out.push(k * 64 + b);
                x &= x - 1;
            }
        }
        out
    }

    pub fn digest(&self) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for &x in &self.w {
            h ^= x;
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
        h
    }
}
