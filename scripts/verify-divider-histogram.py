"""Exact integer reference for the v0.5 divider histogram regression."""
from itertools import count

MASK = 2**64 - 1
SCALE = 2**53


def draws(seed):
    for index in count(1):
        word = (seed + index * 0x9e3779b97f4a7c15) & MASK
        word = ((word ^ (word >> 30)) * 0xbf58476d1ce4e5b9) & MASK
        word = ((word ^ (word >> 27)) * 0x94d049bb133111eb) & MASK
        yield (word ^ (word >> 31)) >> 11


if __name__ == "__main__":
    stream = draws(2026)
    bins = [0] * 6
    for _ in range(16384):
        upper = 950 * SCALE + 100 * next(stream)
        lower = 1900 * SCALE + 200 * next(stream)
        # Compare exact tap voltage to each threshold in tenths of a volt.
        slot = sum(90 * lower >= t * (upper + lower) for t in range(58, 63))
        bins[slot] += 1
    print(bins)
