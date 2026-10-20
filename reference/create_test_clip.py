import os
import sys
import io
import zlib
import struct
import sqlite3
import subprocess

BlockDataBeginChunk = 'BlockDataBeginChunk'.encode('UTF-16BE')
BlockDataEndChunk = 'BlockDataEndChunk'.encode('UTF-16BE')

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
    
    return b.getvalue()

def build_tile_block(block_idx, raw_tile_rgba):
    # raw_tile_rgba is 256*256*4 bytes (RGBA)
    # Check if empty
    has_alpha = any(raw_tile_rgba[3::4])
    if not has_alpha:
        # Empty tile
        block = io.BytesIO()
        block.write(block_idx.to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big')) # has_data = 0
        block_bytes = block.getvalue()
    else:
        # Non-empty tile: 5 channels: 1 alpha + 4 BGRX
        k = 256 * 256
        alpha_channel = bytearray(k)
        bgrx_channel = bytearray(k * 4)
        for i in range(k):
            r = raw_tile_rgba[i*4]
            g = raw_tile_rgba[i*4 + 1]
            b = raw_tile_rgba[i*4 + 2]
            a = raw_tile_rgba[i*4 + 3]
            alpha_channel[i] = a
            bgrx_channel[i*4]     = b
            bgrx_channel[i*4 + 1] = g
            bgrx_channel[i*4 + 2] = r
            bgrx_channel[i*4 + 3] = 0
            
        uncompressed = alpha_channel + bgrx_channel
        compressed = zlib.compress(uncompressed)
        
        block = io.BytesIO()
        block.write(block_idx.to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big'))
        block.write((0).to_bytes(4, 'big'))
        block.write((1).to_bytes(4, 'big')) # has_data = 1
        block.write((len(compressed) + 4).to_bytes(4, 'big')) # BE
        block.write(len(compressed).to_bytes(4, 'little'))    # LE
        block.write(compressed)
        block_bytes = block.getvalue()
        
    total_size = 4 + 4 + len(BlockDataBeginChunk) + len(block_bytes) + 4 + len(BlockDataEndChunk)
    res = io.BytesIO()
    res.write(total_size.to_bytes(4, 'big'))
    res.write((19).to_bytes(4, 'big'))
    res.write(BlockDataBeginChunk)
    res.write(block_bytes)
    res.write((17).to_bytes(4, 'big'))
    res.write(BlockDataEndChunk)
    return res.getvalue()

def build_layer_exta_payload(width, height, layer_rgba):
    grid_w = (width + 255) // 256
    grid_h = (height + 255) // 256
    
    out = io.BytesIO()
    for gy in range(grid_h):
        for gx in range(grid_w):
            block_idx = gy * grid_w + gx
            # Extract 256x256 tile
            tile = bytearray(256 * 256 * 4)
            for ty in range(256):
                cy = gy * 256 + ty
                if cy >= height:
                    continue
                for tx in range(256):
                    cx = gx * 256 + tx
                    if cx >= width:
                        continue
                    src_idx = (cy * width + cx) * 4
                    dst_idx = (ty * 256 + tx) * 4
                    tile[dst_idx:dst_idx+4] = layer_rgba[src_idx:src_idx+4]
            out.write(build_tile_block(block_idx, tile))
            
    return out.getvalue()

def create_test_clip(filename, width=512, height=512):
    # Layer 1: Solid white
    l1_rgba = bytearray(width * height * 4)
    for i in range(width * height):
        l1_rgba[i*4] = 255
        l1_rgba[i*4+1] = 255
        l1_rgba[i*4+2] = 255
        l1_rgba[i*4+3] = 255
        
    # Layer 2: Red square in center (100..400)
    l2_rgba = bytearray(width * height * 4)
    for y in range(100, 400):
        for x in range(100, 400):
            idx = (y * width + x) * 4
            l2_rgba[idx] = 255 # R
            l2_rgba[idx+1] = 0 # G
            l2_rgba[idx+2] = 0 # B
            l2_rgba[idx+3] = 255 # A
            
    exta1_id = b"extrnlid00000000000000000000000000000001"
    exta2_id = b"extrnlid00000000000000000000000000000002"
    
    exta1_data = build_layer_exta_payload(width, height, l1_rgba)
    exta2_data = build_layer_exta_payload(width, height, l2_rgba)
    
    # Build SQLite DB in memory
    db_conn = sqlite3.connect(":memory:")
    cur = db_conn.cursor()
    
    cur.execute('''CREATE TABLE Canvas (
        CanvasId INTEGER PRIMARY KEY,
        CanvasWidth INTEGER,
        CanvasHeight INTEGER,
        CanvasResolution REAL,
        CanvasColorMode INTEGER,
        CanvasRootFolder INTEGER
    )''')
    cur.execute('''CREATE TABLE Layer (
        MainId INTEGER PRIMARY KEY,
        CanvasId INTEGER,
        LayerName TEXT,
        LayerType INTEGER,
        LayerComposite INTEGER,
        LayerOpacity INTEGER,
        LayerClip INTEGER,
        LayerVisibility INTEGER,
        LayerLock INTEGER,
        LayerMasking INTEGER,
        LayerOffsetX INTEGER,
        LayerOffsetY INTEGER,
        LayerRenderOffscrOffsetX INTEGER,
        LayerRenderOffscrOffsetY INTEGER,
        LayerMaskOffsetX INTEGER,
        LayerMaskOffsetY INTEGER,
        LayerMaskOffscrOffsetX INTEGER,
        LayerMaskOffscrOffsetY INTEGER,
        LayerFolder INTEGER,
        LayerFirstChildIndex INTEGER,
        LayerNextIndex INTEGER,
        LayerSelect INTEGER,
        LayerRenderMipmap INTEGER,
        LayerLayerMaskMipmap INTEGER
    )''')
    cur.execute('''CREATE TABLE Mipmap (
        MainId INTEGER PRIMARY KEY,
        BaseMipmapInfo INTEGER
    )''')
    cur.execute('''CREATE TABLE MipmapInfo (
        MainId INTEGER PRIMARY KEY,
        Offscreen INTEGER,
        NextIndex INTEGER
    )''')
    cur.execute('''CREATE TABLE Offscreen (
        MainId INTEGER PRIMARY KEY,
        LayerId INTEGER,
        BlockData TEXT,
        Attribute BLOB
    )''')
    cur.execute('''CREATE TABLE CanvasPreview (
        MainId INTEGER PRIMARY KEY,
        ImageData BLOB
    )''')
    cur.execute('''CREATE TABLE ExternalTableAndColumnName (
        TableName TEXT,
        ColumnName TEXT
    )''')
    
    attr = build_offscreen_attribute(width, height)
    
    cur.execute("INSERT INTO Canvas VALUES (1, ?, ?, 350.0, 1, 1)", (width, height))
    
    # Layer 1 = Root folder (MainId 1)
    # Child 1 = Layer 2 (Background), next = Layer 3 (Foreground)
    cur.execute('''INSERT INTO Layer VALUES (
        1, 1, 'Root', 0, 0, 256, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        1, 2, 0, 0, NULL, NULL
    )''')
    
    # Layer 2 (Background)
    cur.execute('''INSERT INTO Layer VALUES (
        2, 1, 'Background', 1, 0, 256, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 3, 0, 10, NULL
    )''')
    cur.execute("INSERT INTO Mipmap VALUES (10, 100)")
    cur.execute("INSERT INTO MipmapInfo VALUES (100, 1000, 0)")
    cur.execute("INSERT INTO Offscreen VALUES (1000, 2, ?, ?)", (exta1_id, attr))
    
    # Layer 3 (Foreground)
    cur.execute('''INSERT INTO Layer VALUES (
        3, 1, 'Red Square', 1, 0, 256, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1, 20, NULL
    )''')
    cur.execute("INSERT INTO Mipmap VALUES (20, 200)")
    cur.execute("INSERT INTO MipmapInfo VALUES (200, 2000, 0)")
    cur.execute("INSERT INTO Offscreen VALUES (2000, 3, ?, ?)", (exta2_id, attr))
    
    # Canvas Preview (simple 1x1 png or empty)
    # 1x1 png
    png_1x1 = b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15c4\x00\x00\x00\rIDATx\x9cc\xf8\xff\xff?\x00\x05\xfe\x02\xfe\r\xefe\xb4\x00\x00\x00\x00IEND\xaeB`\x82'
    cur.execute("INSERT INTO CanvasPreview VALUES (1, ?)", (png_1x1,))
    cur.execute("INSERT INTO ExternalTableAndColumnName VALUES ('Offscreen', 'BlockData')")
    
    db_conn.commit()
    
    # Export sqlite bytes
    sqlite_bytes = bytearray()
    for line in db_conn.iterdump():
        pass # dump
    # Write db to temp file to read raw bytes
    temp_db_path = "temp_test.sqlite"
    dest = sqlite3.connect(temp_db_path)
    db_conn.backup(dest)
    dest.close()
    db_conn.close()
    
    with open(temp_db_path, "rb") as f:
        sqlite_bytes = f.read()
    os.remove(temp_db_path)
    
    # Assemble CSFCHUNK container
    file_buf = io.BytesIO()
    # 24-byte CSFCHUNK header
    file_buf.write(b"CSFCHUNK")
    file_buf.write((0).to_bytes(8, 'big')) # file size placeholder
    file_buf.write((24).to_bytes(8, 'big')) # offset of first chunk (24)
    
    # Head chunk (40 bytes payload)
    head_pos = file_buf.tell()
    file_buf.write(b"CHNKHead")
    file_buf.write((40).to_bytes(8, 'big'))
    head_payload_pos = file_buf.tell()
    file_buf.write((256).to_bytes(8, 'big')) # version
    sqlite_offset_pos = file_buf.tell()
    file_buf.write((0).to_bytes(8, 'big')) # sqlite pos placeholder
    file_buf.write(b"\x2f\x7b\xf5\x1a\x5b\xc4\x7f\xb0\x10\x84\x7e\x0b\x5e\x5a\x22\xb8") # 16 byte guid
    file_buf.write((0).to_bytes(8, 'big')) # padding to 40 bytes
    
    # Exta chunk 1
    def write_exta(chunk_id, data):
        file_buf.write(b"CHNKExta")
        # Exta payload: u64 id_len (40) + 40-byte id + u64 data_len + data
        exta_payload_len = 8 + len(chunk_id) + 8 + len(data)
        file_buf.write(exta_payload_len.to_bytes(8, 'big'))
        file_buf.write(len(chunk_id).to_bytes(8, 'big'))
        file_buf.write(chunk_id)
        file_buf.write(len(data).to_bytes(8, 'big'))
        file_buf.write(data)
        
    write_exta(exta1_id, exta1_data)
    write_exta(exta2_id, exta2_data)
    
    # SQLi chunk
    sqlite_chunk_pos = file_buf.tell()
    file_buf.write(b"CHNKSQLi")
    file_buf.write(len(sqlite_bytes).to_bytes(8, 'big'))
    file_buf.write(sqlite_bytes)
    
    # Foot chunk
    file_buf.write(b"CHNKFoot")
    file_buf.write((0).to_bytes(8, 'big'))
    
    total_size = file_buf.tell()
    
    # Backpatch CSFCHUNK header file size
    file_buf.seek(8)
    file_buf.write(total_size.to_bytes(8, 'big'))
    
    # Backpatch Head chunk sqlite_offset
    file_buf.seek(sqlite_offset_pos)
    file_buf.write(sqlite_chunk_pos.to_bytes(8, 'big'))
    
    with open(filename, "wb") as f:
        f.write(file_buf.getvalue())
        
    print(f"Generated test clip file '{filename}' with size {total_size} bytes.")

if __name__ == "__main__":
    create_test_clip("test_auth.clip")
