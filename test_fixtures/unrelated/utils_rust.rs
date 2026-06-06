use std::collections::HashMap;

pub fn fibonacci(n: u64) -> u64 {
    let mut memo: HashMap<u64, u64> = HashMap::new();
    fib_memo(n, &mut memo)
}

fn fib_memo(n: u64, memo: &mut HashMap<u64, u64>) -> u64 {
    if n <= 1 {
        return n;
    }
    if let Some(&result) = memo.get(&n) {
        return result;
    }
    let result = fib_memo(n - 1, memo) + fib_memo(n - 2, memo);
    memo.insert(n, result);
    result
}

pub fn is_palindrome(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for i in 0..len / 2 {
        if chars[i] != chars[len - 1 - i] {
            return false;
        }
    }
    true
}
