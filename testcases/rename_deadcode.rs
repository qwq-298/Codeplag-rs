// rename + deadcode: variables renamed + extra useless functions added
pub fn my_sort(data: &mut [i32]) {
    let size = data.len();
    for x in 0..size {
        for y in 0..size - x - 1 {
            if data[y] > data[y + 1] {
                data.swap(y, y + 1);
            }
        }
    }
}

// Dead code — should not affect similarity of the real functions
pub fn unused_helper() -> i32 {
    let x = 42;
    x + 10
}

pub fn maximum_value(elements: &[i32]) -> Option<i32> {
    if elements.is_empty() {
        return None;
    }
    let mut biggest = elements[0];
    for &e in elements.iter().skip(1) {
        if e > biggest {
            biggest = e;
        }
    }
    Some(biggest)
}

pub fn deprecated_sort(v: &mut Vec<i32>) {
    for _ in 0..v.len() {}
}
