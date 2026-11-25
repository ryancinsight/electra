"""70-digit nodal oracle for the RC ladder; source drive and output current injection."""
from decimal import Decimal as D, getcontext

getcontext().prec = 70
TAU = D("6.2831853071795864769252867665590057683943387987502116419498891846156328")


def solve_ladder(stages, frequency):
    # Real block representation of G + jB. Two RHS columns solve independent
    # experiments: a 1 V source for H, and a 1 A output injection for Zout.
    n = len(stages)
    size = 2 * n
    matrix = [[D(0) for _ in range(size + 2)] for _ in range(size)]
    for j, (r, c, load) in enumerate(stages):
        g = 1 / D(r)
        b = TAU * frequency * D(c)
        for offset in (0, n):
            row = offset + j
            matrix[row][row] += g + D(load)
            if j:
                matrix[row - 1][row - 1] += g
                matrix[row][row - 1] -= g
                matrix[row - 1][row] -= g
        matrix[j][j + n] -= b
        matrix[j + n][j] += b
    matrix[0][-2] = 1 / D(stages[0][0])
    matrix[n - 1][-1] = 1
    eliminate(matrix)
    # One tuple of (real, imaginary) node voltages per excitation.
    return tuple([(matrix[i][col], matrix[n + i][col]) for i in range(n)]
                 for col in (-2, -1))


def eliminate(matrix):
    """Pivot the square coefficient block and solve all appended right-hand sides in place."""
    size = len(matrix)
    for j in range(size):
        k = max(range(j, size), key=lambda i: abs(matrix[i][j]))
        matrix[j], matrix[k] = matrix[k], matrix[j]
        pivot = matrix[j][j]
        matrix[j] = [x / pivot for x in matrix[j]]
        for i, row in enumerate(matrix):
            if i != j and row[j]:
                factor = row[j]
                matrix[i] = [x - factor * y for x, y in zip(row, matrix[j])]
