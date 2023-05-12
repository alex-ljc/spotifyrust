import json

def print_all_keys(filename):
    with open(filename, 'r') as f:
        hashmap = json.load(f)
        hashmap_temp = {}
        for value in hashmap.values():
            if "Huh Gak" in value["name"]:
                print(value['name'])
                

print_all_keys("albums.json")