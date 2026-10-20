with open(r'C:/ArtCraft/reference/clip_to_psd.py', 'r', encoding='utf-8') as f:
    lines = f.readlines()

for i, l in enumerate(lines):
    if 'attributes_arrays' in l or 'pixel_packing_params' in l or 'first_packing_channel_count' in l:
        print(f"{i+1}: {l.strip()}")
