import json
from src.transformer import csv_to_json


def test_basic_csv():
    csv_text = "name,age\nAda,12\nBob,13\n"
    out = csv_to_json(csv_text)
    parsed = json.loads(out)
    assert parsed == [{"name": "Ada", "age": "12"}, {"name": "Bob", "age": "13"}]
