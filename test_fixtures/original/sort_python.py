def bubble_sort(arr):
    n = len(arr)
    for i in range(n):
        for j in range(n - i - 1):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr


def find_max(arr):
    if not arr:
        return None
    max_val = arr[0]
    for item in arr[1:]:
        if item > max_val:
            max_val = item
    return max_val
