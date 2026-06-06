function mySort(data) {
    /////////////////sort 方法
    let size = data.length;
    for (let x = 0; x < size; x++) {

        for (let y = 0; y < size - x - 1; y++) {

            if (data[y] > data[y + 1]) {
                [data[y], data[y + 1]] = [data[y + 1], data[y]];
            }

        }
    }

    return data;
}

function maximumValue(elements) {
    if (elements.length === 0) {
        return null;///////////////////空数组时返回 null
    }
    let biggest = elements[0];
    for (let e of elements.slice(1)) {
        if (e > biggest) {
            biggest = e;
        }
    }
    return biggest;
}
