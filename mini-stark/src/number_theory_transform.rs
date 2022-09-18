extern crate test;

use crate::polynomial::MultivariatePolynomial;
use crate::polynomial::Polynomial;
use legacy_algebra::batch_inverse;
use legacy_algebra::ExtensionOf;
use legacy_algebra::Felt;
use legacy_algebra::StarkFelt;
use std::primitive;

// TODO: fix types. E might have to have the same BaseField as F.
pub fn ntt<F, E>(primitive_root: F, values: &[E]) -> Vec<E>
where
    F: StarkFelt,
    E: Felt<BaseFelt = F>,
{
    if values.len() <= 1 {
        return values.to_vec();
    }

    assert!(values.len().is_power_of_two());
    assert_eq!(primitive_root.pow(&[values.len() as u64]), F::one());
    let half = values.len() / 2;
    assert_ne!(primitive_root.pow(&[half as u64]), F::one());

    let odd_values = values
        .iter()
        .skip(1)
        .step_by(2)
        .copied()
        .collect::<Vec<E>>();
    let even_values = values.iter().step_by(2).copied().collect::<Vec<E>>();

    let odds = ntt(primitive_root.square(), &odd_values);
    let evens = ntt(primitive_root.square(), &even_values);

    (0..values.len())
        .map(|i| evens[i % half] + odds[i % half].mul_base(primitive_root.pow(&[i as u64])))
        .collect()
}

pub fn number_theory_transform<E>(values: &[E]) -> Vec<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    let primitive_root = E::BaseFelt::get_root_of_unity(values.len().ilog2());
    ntt(primitive_root, values)
}

pub fn inverse_number_theory_transform<E>(values: &[E]) -> Vec<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    // let ninv = E::from(values.len() as u32).inverse().unwrap();
    // // Inverse primitive root to calculate in reverse order
    // let transformed_values = ntt(primitive_root.inverse().unwrap(), values);
    // transformed_values
    //     .into_iter()
    //     .map(|transformed_value| ninv * transformed_value)
    //     .collect()
    let ninv = E::from(values.len() as u32).inverse().unwrap();
    let primitive_root = E::BaseFelt::get_root_of_unity(values.len().ilog2());
    ntt(primitive_root.inverse().unwrap(), values)
        .into_iter()
        .map(|v| ninv * v)
        .collect()
}

pub fn fast_multiply<E>(lhs: &Polynomial<E>, rhs: &Polynomial<E>) -> Polynomial<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    if lhs.is_zero() || rhs.is_zero() {
        return Polynomial::new(vec![]);
    }

    let degree = (lhs.degree() + rhs.degree()) as usize;

    if degree < 8 {
        return lhs.clone() * rhs.clone();
    }

    let n = degree.next_power_of_two();
    // let root = E::get_root_of_unity(degree.ilog2() + 1);
    // assert_eq!(root.pow(&[order as u64]), E::one());
    // assert_ne!(root.pow(&[(order / 2) as u64]), E::one());

    let mut lhs_coefficients = vec![E::zero(); n];
    lhs_coefficients[..lhs.coefficients.len()].copy_from_slice(&lhs.coefficients);
    // lhs.coefficients[..(lhs.degree() as usize + 1)].to_vec();
    // while lhs_coefficients.len() < n {
    //     lhs_coefficients.push(E::zero());
    // }

    let mut rhs_coefficients = vec![E::zero(); n];
    rhs_coefficients[..rhs.coefficients.len()].copy_from_slice(&rhs.coefficients);
    // rhs.coefficients[..(rhs.degree() as usize + 1)].to_vec();
    // while rhs_coefficients.len() < n {
    //     rhs_coefficients.push(E::zero());
    // }

    let lhs_codeword = number_theory_transform(&lhs_coefficients);
    let rhs_codeword = number_theory_transform(&rhs_coefficients);

    let hadamard_product = lhs_codeword
        .into_iter()
        .zip(rhs_codeword)
        .map(|(l, r)| l * r)
        .collect::<Vec<E>>();
    let product_coefficients = inverse_number_theory_transform(&hadamard_product);

    // Polynomial::new(product_coefficients[..(degree + 1)].to_vec())
    Polynomial::new(product_coefficients)
}

pub fn fast_zerofier<E>(domain: &[E]) -> Polynomial<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    // assert_eq!(
    //     primitive_root.pow(&[root_order as u64]),
    //     E::one(),
    //     "supplied root does not have supplied order"
    // );
    // assert_ne!(
    //     primitive_root.pow(&[(root_order / 2) as u64]),
    //     E::one(),
    //     "supplied root is not primitive root of supplied order"
    // );

    if domain.is_empty() {
        return Polynomial::new(vec![E::zero()]);
    }

    if domain.len() == 1 {
        return Polynomial::new(vec![-domain[0], E::one()]);
    }

    let half = domain.len() / 2;
    let left = fast_zerofier(&domain[..half]);
    let right = fast_zerofier(&domain[half..]);
    fast_multiply(&left, &right)
}

pub fn fast_evaluate_symbolic<E>(
    polynomial: &MultivariatePolynomial<E>,
    point: &[Polynomial<E>],
    primitive_root: E,
    root_order: usize,
) -> Polynomial<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    let mut accumulator = Polynomial::new(vec![]);
    for (pad, coefficient) in polynomial.powers.iter().zip(polynomial.coefficients.iter()) {
        let mut product = Polynomial::new(vec![*coefficient]);
        for (i, power) in pad.iter().enumerate() {
            product = fast_multiply(&product, &(point[i].clone() ^ *power));
            // product = product * (point[i].clone() ^ *power);
        }
        accumulator = accumulator + product;
    }
    accumulator
}

fn fast_evaluate_domain<E>(polynomial: &Polynomial<E>, domain: &[E]) -> Vec<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    if domain.is_empty() {
        return vec![];
    }

    if domain.len() == 1 {
        return vec![polynomial.evaluate(domain[0])];
    }

    let half = domain.len() / 2;

    let left_zerofier = fast_zerofier(&domain[..half]);
    let right_zerofier = fast_zerofier(&domain[half..]);

    let left = fast_evaluate_domain(
        &(polynomial.clone() % left_zerofier),
        &domain[..half].to_vec(),
    );
    let right = fast_evaluate_domain(
        &(polynomial.clone() % right_zerofier),
        &domain[half..].to_vec(),
    );

    left.into_iter().chain(right.into_iter()).collect()
}

pub fn fast_interpolate<E>(domain: &[E], values: &[E]) -> Polynomial<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    assert_eq!(
        domain.len(),
        values.len(),
        "cannot interpolate over domain of different length than values list"
    );

    if domain.is_empty() {
        return Polynomial::new(vec![]);
    }

    if domain.len() == 1 {
        return Polynomial::new(vec![values[0]]);
    }

    let half = domain.len() / 2;

    let left_zerofier = fast_zerofier(&domain[..half].to_vec());
    let right_zerofier = fast_zerofier(&domain[half..].to_vec());

    let left_offset = fast_evaluate_domain(&right_zerofier, &domain[..half].to_vec());
    let right_offset = fast_evaluate_domain(&left_zerofier, &domain[half..].to_vec());

    // if not all(not v.is_zero() for v in left_offset):
    //     print("left_offset:", " ".join(str(v) for v in left_offset))

    let left_offset_inverse = batch_inverse(&left_offset);
    let left_targets = left_offset_inverse
        .into_iter()
        .zip(values[..half].iter().copied())
        .map(|(inverse_denominator, numerator)| numerator * inverse_denominator.unwrap())
        .collect::<Vec<E>>();
    let right_offset_inverse = batch_inverse(&right_offset);
    let right_targets = right_offset_inverse
        .into_iter()
        .zip(values[half..].iter().copied())
        .map(|(inverse_denominator, numerator)| numerator * inverse_denominator.unwrap())
        .collect::<Vec<E>>();

    let left_interpolant = fast_interpolate(&domain[..half].to_vec(), &left_targets);
    let right_interpolant = fast_interpolate(&domain[half..].to_vec(), &right_targets);

    fast_multiply(&left_interpolant, &right_zerofier)
        + fast_multiply(&right_interpolant, &left_zerofier)

    // left_interpolant * right_zerofier + right_interpolant * left_zerofier
}

pub fn fast_coset_evaluate<E>(polynomial: &Polynomial<E>, offset: E, order: usize) -> Vec<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    let scaled_polynomial = polynomial.scale(offset);
    let mut scaled_coefficients = scaled_polynomial.coefficients;
    while scaled_coefficients.len() < order {
        scaled_coefficients.push(E::zero());
    }
    number_theory_transform(&scaled_coefficients)
}

// Clean division only
pub fn fast_coset_divide<E>(
    lhs: &Polynomial<E>,
    rhs: &Polynomial<E>,
    offset: E,
    primitive_root: E,
    root_order: usize,
) -> Polynomial<E>
where
    E: Felt,
    E::BaseFelt: StarkFelt,
{
    assert_eq!(
        primitive_root.pow(&[root_order as u64]),
        E::one(),
        "supplied root does not have supplied order"
    );
    assert_ne!(
        primitive_root.pow(&[(root_order / 2) as u64]),
        E::one(),
        "supplied root is not primitive root of supplied order"
    );
    assert!(!rhs.is_zero(), "cannot divide by zero polynomial");

    if lhs.is_zero() {
        return Polynomial::new(vec![]);
    }

    assert!(
        rhs.degree() <= lhs.degree(),
        "cannot divide by polynomial of larger degree",
    );

    let mut root = primitive_root;
    let mut order = root_order;
    let degree = lhs.degree().max(rhs.degree());

    if degree < 8 {
        return lhs.clone() / rhs.clone();
    }

    while degree < (order / 2) as isize {
        root.square_in_place();
        order /= 2;
    }

    let scaled_lhs = lhs.clone().scale(offset);
    let scaled_rhs = rhs.clone().scale(offset);

    let mut lhs_coefficients = scaled_lhs.coefficients[..((lhs.degree() + 1) as usize)].to_vec();
    while lhs_coefficients.len() < order {
        lhs_coefficients.push(E::zero());
    }

    let mut rhs_coefficients = scaled_rhs.coefficients[..((rhs.degree() + 1) as usize)].to_vec();
    while rhs_coefficients.len() < order {
        rhs_coefficients.push(E::zero());
    }

    let lhs_codeword = number_theory_transform(&lhs_coefficients);
    let rhs_codeword = number_theory_transform(&rhs_coefficients);
    let rhs_codeword_inverse = batch_inverse(&rhs_codeword);
    let quotient_codeword = lhs_codeword
        .into_iter()
        .zip(rhs_codeword_inverse)
        .map(|(numerator, denominator_inverse)| numerator * denominator_inverse.unwrap())
        .collect::<Vec<E>>();

    let scaled_quotient_coefficients = inverse_number_theory_transform(&quotient_codeword);
    let scaled_quotient = Polynomial::new(scaled_quotient_coefficients);

    scaled_quotient.scale(offset.inverse().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use legacy_algebra::fp_u128::BaseFelt;
    use legacy_algebra::Felt;
    use num_traits::One;
    use num_traits::Zero;
    use test::Bencher;

    #[test]
    fn test_number_theory_transform() {
        let primitive_root = BaseFelt::get_root_of_unity(1);

        println!(
            "{}, {}, {}, {}",
            primitive_root,
            primitive_root.pow(&[2]),
            primitive_root.pow(&[3]),
            primitive_root.pow(&[4])
        );

        println!(
            "{:?}",
            number_theory_transform(&[BaseFelt::zero(), BaseFelt::one()])
        );
    }

    #[ignore]
    #[bench]
    fn bench_interpolate_100_points(b: &mut Bencher) {
        let points = 100;
        let domain = (0u64..points)
            .map(|i| BaseFelt::GENERATOR.pow(&[i]))
            .collect::<Vec<_>>();
        let values = (0u64..points).map(BaseFelt::from).collect::<Vec<_>>();
        let root_order = (domain.len() + 1).next_power_of_two();
        let primitive_root = BaseFelt::get_root_of_unity(root_order.ilog2());

        b.iter(|| fast_interpolate(&domain, &values))
    }
}
