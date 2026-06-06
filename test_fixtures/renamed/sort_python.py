def my_sort(lst):
    length = len(lst)
    for x in range(length):
        for y in range(length - x - 1):
            if lst[y] > lst[y + 1]:
                lst[y], lst[y + 1] = lst[y + 1], lst[y]
    return lst


def maximum_value(lst):
    if not lst:
        return None
    biggest = lst[0]
    for element in lst[1:]:
        if element > biggest:
            biggest = element
    return biggest
