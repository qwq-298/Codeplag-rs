package main

func mySort(data []int) {
    size := len(data)
    for x := 0; x < size; x++ {
        for y := 0; y < size-x-1; y++ {
            if data[y] > data[y+1] {
                data[y], data[y+1] = data[y+1], data[y]
            }
        }
    }
}

func maximumValue(elements []int) (int, bool) {
    if len(elements) == 0 {
        return 0, false
    }
    biggest := elements[0]
    for _, e := range elements[1:] {
        if e > biggest {
            biggest = e
        }
    }
    return biggest, true
}
