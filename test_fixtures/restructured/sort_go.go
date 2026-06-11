package main

func bubbleSort(arr []int) {
    n := len(arr)
    for i := 0; i < n-1; i++ {
        swapped := false
        for j := 0; j < n-i-1; j++ {
            if arr[j] > arr[j+1] {
                arr[j], arr[j+1] = arr[j+1], arr[j]
                swapped = true
            }
        }
        if !swapped {
            break
        }
    }
}

func findMax(arr []int) int {
    if len(arr) == 0 {
        return -1
    }
    max := arr[0]
    for _, v := range arr {
        if v > max {
            max = v
        }
    }
    return max
}
