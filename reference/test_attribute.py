import sys
import io

# Import from clip_to_psd
from clip_to_psd import parse_offscreen_attributes_sql_value

def build_offscreen_attribute(width, height):
    grid_w = (width + 255) // 256
    grid_h = (height + 255) // 256
    
    b = io.BytesIO()
    # Header (16 bytes)
    b.write((16).to_bytes(4, 'big'))
    b.write((102).to_bytes(4, 'big')) # info_section_size
    b.write((42).to_bytes(4, 'big'))  # extra_info_section_size
    b.write((0).to_bytes(4, 'big'))
    
    # "Parameter" (4 + 18 = 22 bytes)
    param_str = 'Parameter'.encode('UTF-16BE')
    b.write(len('Parameter').to_bytes(4, 'big'))
    b.write(param_str)
    
    # Dimensions (16 bytes)
    b.write(width.to_bytes(4, 'big'))
    b.write(height.to_bytes(4, 'big'))
    b.write(grid_w.to_bytes(4, 'big'))
    b.write(grid_h.to_bytes(4, 'big'))
    
    # Attributes array (16 ints = 64 bytes)
    attrs = [0] * 16
    attrs[1] = 1 # first_packing_channel_count (alpha)
    attrs[2] = 4 # second_packing_channel_count (bgrx)
    attrs[8] = 8 # 8-bit
    for val in attrs:
        b.write(val.to_bytes(4, 'big'))
        
    # "InitColor" (4 + 18 = 22 bytes)
    init_str = 'InitColor'.encode('UTF-16BE')
    b.write(len('InitColor').to_bytes(4, 'big'))
    b.write(init_str)
    
    # 5 ints (20 bytes)
    b.write((0).to_bytes(4, 'big'))
    b.write((0).to_bytes(4, 'big')) # default_fill_black_white
    b.write((0).to_bytes(4, 'big'))
    b.write((0).to_bytes(4, 'big'))
    b.write((0).to_bytes(4, 'big'))
    
    data = b.getvalue()
    assert len(data) == 16 + 102 + 42, f"Expected 160 bytes, got {len(data)}"
    return data

raw = build_offscreen_attribute(1920, 1080)
print(f"Generated attribute len: {len(raw)}")
parsed = parse_offscreen_attributes_sql_value(raw)
print(f"Parsed successfully: {parsed}")
