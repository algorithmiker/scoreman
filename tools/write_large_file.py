#!/usr/bin/env python3
import sys

input_filename = sys.argv[1]
out_name = sys.argv[2]
desired_MiB = int(sys.argv[3]) * 1024 * 1024
out = open(out_name, "w")
inp = open(input_filename)
c = inp.read()
inp.close()

size_written = 0
while size_written < desired_MiB:
    print(c, file=out)
    size_written += len(c)
