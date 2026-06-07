// rename + comments: variables renamed + inline comments added
// 冒泡排序函数 — 核心算法
pub fn my_sort(data: &mut [i32]) {
    let size = data.len();          // 获取数组长度
    for x in 0..size {              // 外层循环
        for y in 0..size - x - 1 {  // 内层循环
            if data[y] > data[y + 1] {  // 比较相邻元素
                data.swap(y, y + 1);    // 交换位置
            }
        }
    }
}

// 查找最大值函数
pub fn maximum_value(elements: &[i32]) -> Option<i32> {
    if elements.is_empty() {     // 空数组检查
        return None;             // 返回空值
    }
    let mut biggest = elements[0];   // 初始化最大值
    for &e in elements.iter().skip(1) {  // 遍历剩余元素
        if e > biggest {           // 比较
            biggest = e;           // 更新最大值
        }
    }
    Some(biggest)                  // 返回结果
}
