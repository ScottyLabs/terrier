use ndarray::Array2;
use statrs::distribution::{Continuous, ContinuousCDF, Normal};
use std::error::Error;

#[derive(Clone)]
pub struct Rank {
    pub means: Vec<f64>,
    ranks: Array2<i32>,
}

impl Rank {
    pub fn new(means: Vec<f64>, ranks: Array2<i32>) -> Self {
        Rank { means, ranks }
    }

    fn cost_function(&self, means: &[f64]) -> f64 {
        let (n, _) = self.ranks.dim();
        let mut cost = 0.0;
        let normal = Normal::new(0.0, 1.0).unwrap();

        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                // self.ranks[[i, j]] is count of i beating j
                // Probability model: P(i beats j) = Phi(mu_i - mu_j)
                // We maximize likelihood -> minimize negative log likelihood
                // Cost += count * log2(Phi(mu_i - mu_j))
                let prob = normal.cdf(means[i] - means[j]);
                if prob > 1e-9 {
                    cost += (self.ranks[[i, j]] as f64) * prob.log2();
                } else {
                    // Penalty for very low probability to avoid -inf
                    cost += (self.ranks[[i, j]] as f64) * -30.0;
                }
            }
        }

        // Regularization / Prior: means ~ N(0, 1)
        // cost -= means[i] * means[i] / 2.0; (log of Gaussian prior)
        for i in 0..n {
            cost -= means[i] * means[i] / 2.0;
        }

        -cost
    }

    pub fn calc_expected_means(&mut self) {
        let n = self.means.len();
        if n == 0 {
            return;
        }

        let mut current_means = self.means.clone();
        let mut current_cost = self.cost_function(&current_means);

        let initial_learning_rate = 0.1;
        let mut learning_rate = initial_learning_rate;
        let iterations = 200;
        let epsilon = 1e-4;

        for _ in 0..iterations {
            let mut gradient = vec![0.0; n];
            let h = 1e-5;

            // Compute gradient using finite differences
            for i in 0..n {
                let mut means_plus = current_means.clone();
                means_plus[i] += h;
                let cost_plus = self.cost_function(&means_plus);

                // Using forward difference roughly, or central? Central is better.
                let mut means_minus = current_means.clone();
                means_minus[i] -= h;
                let cost_minus = self.cost_function(&means_minus);

                gradient[i] = (cost_plus - cost_minus) / (2.0 * h);
            }

            // Update
            let mut next_means = current_means.clone();
            for i in 0..n {
                next_means[i] -= learning_rate * gradient[i];
            }

            let next_cost = self.cost_function(&next_means);

            if next_cost < current_cost {
                // Determine meaningful improvement
                if (current_cost - next_cost).abs() < epsilon {
                    current_means = next_means;
                    break;
                }
                current_means = next_means;
                current_cost = next_cost;
                // boost learning rate slightly? Or keep it.
            } else {
                // Cost increased, reduce step size
                learning_rate *= 0.5;
            }

            if learning_rate < 1e-6 {
                break;
            }
        }

        self.means = current_means;
    }

    pub fn rank(&self) -> Result<Vec<usize>, Box<dyn Error>> {
        let mut indices: Vec<usize> = (0..self.means.len()).collect();
        indices.sort_by(|&i, &j| self.means[j].partial_cmp(&self.means[i]).unwrap());
        Ok(indices)
    }
}
