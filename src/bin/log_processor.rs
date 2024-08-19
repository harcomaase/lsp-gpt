use std::fs;

fn main() {
    let file_contents = fs::read_to_string("./invocations.csv").unwrap();
    let values: Vec<f64> = file_contents
        .lines()
        .skip(526)
        .filter(|line|line.contains("diagnostic"))
        .map(|line| line.split(',').nth(2).unwrap().parse().unwrap())
        .collect();
    let sum: f64 = values.iter().sum();
    let quantity = values.len() as f64;
    let average = sum / quantity;

    let sum_of_deviations: f64 = values.iter().map(|v| v - average).map(|v| v * v).sum();
    let variance = sum_of_deviations / quantity;
    let standard_deviation = variance.sqrt();

    println!("quantity: {quantity}");
    println!("average: {average}");
    println!("variance: {variance}");
    println!("standard_deviation: {standard_deviation}");
}
