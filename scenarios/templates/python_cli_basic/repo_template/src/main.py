import sys
from .transformer import csv_to_json


def main(argv=None):
    argv = argv or sys.argv[1:]
    data = sys.stdin.read()
    out = csv_to_json(data)
    sys.stdout.write(out)


if __name__ == "__main__":
    main()
