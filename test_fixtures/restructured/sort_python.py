def bubble_sort(arr):
    n = len(arr)
    swapped = True
    k = 0
    while swapped:
        swapped = False
        for i in range(n - k - 1):
            if arr[i] > arr[i + 1]:
                arr[i], arr[i + 1] = arr[i + 1], arr[i]
                swapped = True
        k += 1
    return arr


def find_max(arr):
    return max(arr) if arr else None
