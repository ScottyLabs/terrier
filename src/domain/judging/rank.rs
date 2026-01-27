use ndarray::{Array2, array};
use scirs2_optimize::unconstrained::{Method, minimize};
use scirs2_stats::distributions;
use std::error::Error;

#[derive(Clone)]
pub struct Rank {
    means: Vec<f64>,
    ranks: Array2<i32>,
}

impl Rank {
    pub fn new(means: Vec<f64>, ranks: Array2<i32>) -> Self {
        Rank { means, ranks }
    }

    fn cost_function(&self, means: &Vec<f64>) -> f64 {
        let (n, _) = self.ranks.dim();

        let mut cost = 0.0;
        let normal = distributions::Normal::new(0.0, 1.0).unwrap();

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                };
                cost += (self.ranks[[i, j]] as f64) * normal.cdf(means[i] - means[j]).log2();
            }
        }

        // subtraction
        for i in 0..n {
            cost -= means[i] * means[i] / 2.0;
        }

        -cost
    }

    fn calc_expected_means(&mut self) {
        let result = minimize(
            |means| self.cost_function(&means.to_vec()),
            &self.means,
            Method::BFGS,
            None,
        )
        .unwrap();

        self.means = result.x.to_vec();
    }

    pub fn rank(&self) -> Result<Vec<usize>, Box<dyn Error>> {
        let mut indices: Vec<usize> = (0..self.means.len()).collect();
        indices.sort_by(|&i, &j| self.means[j].partial_cmp(&self.means[i]).unwrap());
        Ok(indices)
    }
}
