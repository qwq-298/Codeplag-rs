int my_sort(int data[], int size) {
    for (int x = 0; x < size; x++) {
        for (int y = 0; y < size - x - 1; y++) {
            if (data[y] > data[y + 1]) {
                int tmp = data[y];
                data[y] = data[y + 1];
                data[y + 1] = tmp;
            }
        }
    }
    return 0;
}

int maximum_value(int elements[], int size) {
    if (size == 0) return -1;
    int biggest = elements[0];
    for (int i = 1; i < size; i++) {
        if (elements[i] > biggest) biggest = elements[i];
    }
    return biggest;
}
