import time
import itertools
from random import randint
from bitstring import Bits

def hamming_distance(a, b):
    assert(len(a) == len(b))
    res = len(a)
    for (x, y) in zip(a, b):
        if x == y:
            res -= 1
    return res

def to_bits(x):
    return [Bits(uint=i, length=8) for i in x]

n = 9 #int(sys.argv[1])

start_time = time.time()

smallest = 0
s = []
for _ in range(10000000):
    start = []
    for _ in range(n):
        x = randint(0, 255)
        while x in start:
            x = randint(0, 255)
        start.append(x)

    h = min(map(lambda a: hamming_distance(a[0], a[1]), itertools.combinations(to_bits(start), 2)))
    if h > smallest:
        print(f"Distance {h} with {start}")
        smallest = h
        s = start
    
    if smallest == 4:
        break

print(f"Distance of {smallest} with {s}")
print(f"In {time.time()-start_time}s")