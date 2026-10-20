with open(r'C:/ArtCraft/reference/clip_to_psd.py', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, l in enumerate(lines):
    if 'BlockData' in l or 'external_id' in l:
        print(f"{i+1}: {l.strip()}")
