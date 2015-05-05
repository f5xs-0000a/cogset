use {Euclid, Point};
use point::Euclidean;

/// Clustering via the *k*-means algorithm (aka Lloyd's algorithm).
///
/// > *k*-means clustering aims to partition *n* observations into *k*
/// clusters in which each observation belongs to the cluster with the
/// nearest mean, serving as a prototype of the cluster.<sup><a
/// href="https://en.wikipedia.org/wiki/K-means_clustering">wikipedia</a></sup>
///
/// This is a heuristic, iterative approximation to the true optimal
/// assignment. The parameters used to control the approximation can
/// be set via `KmeansBuilder`.
///
/// # Examples
///
/// ```rust
/// use cogset::{Euclid, Kmeans};
///
/// let data = [Euclid([0.0, 0.0]),
///             Euclid([1.0, 0.5]),
///             Euclid([0.2, 0.2]),
///             Euclid([0.3, 0.8]),
///             Euclid([0.0, 1.0])];
/// let k = 3;
///
/// let kmeans = Kmeans::new(&data, k);
///
/// println!("{:?}", kmeans.clusters());
/// ```

pub struct Kmeans<T> {
    assignments: Vec<usize>,
    centres: Vec<Euclid<T>>,
    iterations: usize,
    converged: bool,
}

impl<T> Kmeans<T>
    where Euclid<T>: Point + Euclidean + Clone
{
    /// Run k-means on `data` with the default settings.
    pub fn new(data: &[Euclid<T>], k: usize) -> Kmeans<T> {
        KmeansBuilder::new().kmeans(data, k)
    }

    /// Retrieve the means and the clusters themselves that this
    /// *k*-means instance computed.
    ///
    /// The clusters are represented by vectors of indexes into the
    /// original data.
    pub fn clusters(&self) -> Vec<(Euclid<T>, Vec<usize>)> {
        let mut ret = self.centres.iter().cloned().map(|c| (c, vec![])).collect::<Vec<_>>();

        for (idx, &assign) in self.assignments.iter().enumerate() {
            ret[assign].1.push(idx);
        }

        ret
    }

    /// Return whether the algorithm converged, and how many steps
    /// that took.
    ///
    /// `Ok` is returned if the algorithm did meet the tolerance
    /// criterion, and `Err` if it reached the iteration limit
    /// instead.
    pub fn converged(&self) -> Result<usize, usize> {
        if self.converged {
            Ok(self.iterations)
        } else {
            Err(self.iterations)
        }
    }
}

const DEFAULT_MAX_ITER: usize = 100;
const DEFAULT_TOL: f64 = 1e-6;

/// A builder for *k*-means to provide control over parameters for the
/// algorithm.
///
/// This allows one to tweak settings like the tolerance and the
/// number of iterations.
///
/// # Examples
///
/// ```rust
/// use cogset::{Euclid, KmeansBuilder};
///
/// let data = [Euclid([0.0, 0.0]),
///             Euclid([1.0, 0.5]),
///             Euclid([0.2, 0.2]),
///             Euclid([0.3, 0.8]),
///             Euclid([0.0, 1.0])];
///
/// let k = 3;
///
/// // we want the means extra precise.
/// let tol = 1e-12;
/// let kmeans = KmeansBuilder::new().tolerance(tol).kmeans(&data, k);
///
/// println!("{:?}", kmeans.clusters());
/// ```
pub struct KmeansBuilder {
    tol: f64,
    max_iter: usize,
}

impl KmeansBuilder {
    /// Create a default `KmeansBuilder`
    pub fn new() -> KmeansBuilder {
        KmeansBuilder {
            tol: DEFAULT_TOL,
            max_iter: DEFAULT_MAX_ITER,
        }
    }

    /// Set the tolerance used to decide if the iteration has
    /// converged to `tol`.
    pub fn tolerance(self, tol: f64) -> KmeansBuilder {
        KmeansBuilder { tol: tol, .. self }
    }
    /// Set the maximum number of iterations to run before aborting to
    /// `max_iter`.
    pub fn max_iter(self, max_iter: usize) -> KmeansBuilder {
        KmeansBuilder { max_iter: max_iter, .. self }
    }

    /// Run *k*-means with the given settings.
    ///
    /// This is functionally identical to `Kmeans::new`, other than
    /// the internal parameters differing.
    pub fn kmeans<T>(self, data: &[Euclid<T>], k: usize) -> Kmeans<T>
        where Euclid<T>: Point + Euclidean + Clone
    {
        assert!(2 <= k && k < data.len());

        let n = data.len();
        let mut assignments = vec![!0; n];
        let mut costs = vec![0.0; n];
        let mut centres = data.iter().take(k).cloned().collect::<Vec<_>>();
        let mut counts = vec![0; k];
        update_assignments(data, &mut assignments, &mut counts, &mut costs, &centres);
        let mut objective = costs.iter().fold(0.0, |a, b| a + *b);

        let mut converged = false;
        let mut iter = 0;
        while iter < self.max_iter {
            update_centres(data, &assignments, &counts, &mut centres);
            update_assignments(data, &mut assignments, &mut counts, &mut costs, &centres);

            let new_objective = costs.iter().fold(0.0, |a, b| a + *b);

            if (new_objective - objective).abs() < self.tol {
                converged = true;
                break
            }

            objective = new_objective;
            iter += 1
        }

        Kmeans {
            assignments: assignments,
            centres: centres,
            iterations: iter,
            converged: converged,
        }
    }
}


fn update_assignments<T>(data: &[Euclid<T>],
                         assignments: &mut [usize], counts: &mut [usize], costs: &mut [f64],
                         centres: &[Euclid<T>])
    where Euclid<T>: Point + Euclidean + Clone
{
    use std::f64::INFINITY as INF;

    for place in counts.iter_mut() { *place = 0 }

    for ((point, assign), cost) in data.iter().zip(assignments.iter_mut()).zip(costs.iter_mut()) {
        let mut min_dist = INF;
        let mut index = 0;
        for (i, c) in centres.iter().enumerate() {
            let dist = point.dist(c);
            if dist < min_dist {
                min_dist = dist;
                index = i;
            }
        }

        *cost = min_dist;
        *assign = index;
        counts[index] += 1;
    }
}

fn update_centres<T>(data: &[Euclid<T>],
                     assignments: &[usize], counts: &[usize],
                     centres: &mut [Euclid<T>])
    where Euclid<T>: Point + Euclidean + Clone
{
    for place in centres.iter_mut() { *place = <Euclid<T>>::zero() }

    for (point, assign) in data.iter().zip(assignments.iter()) {
        centres[*assign].add(point)
    }
    for (place, scale) in centres.iter_mut().zip(counts.iter()) {
        place.scale(1.0 / *scale as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Euclid;

    #[test]
    fn smoke() {
        let points = [Euclid([0.0, 0.0]),
                      Euclid([1.0, 0.5]),
                      Euclid([0.2, 0.2]),
                      Euclid([0.3, 0.8]),
                      Euclid([0.0, 1.0]),
                      ];

        let res = Kmeans::new(&points, 3);
        let mut clusters = res.clusters();
        for &mut (_, ref mut v) in &mut clusters {
            v.sort()
        }
        clusters.sort_by(|a, b| a.1.cmp(&b.1));
        assert_eq!(clusters,
                   [(Euclid([0.1, 0.1]), vec![0, 2]),
                    (Euclid([1.0, 0.5]), vec![1]),
                    (Euclid([0.15, 0.9]), vec![3, 4])]);

        assert_eq!(res.converged(), Ok(2));
    }
}
