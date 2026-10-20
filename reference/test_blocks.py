import sys
import io
import zlib
from clip_to_psd import parse_chunk_with_blocks, BlockDataBeginChunk, BlockDataEndChunk

def create_tile_block(block_idx, compressed_data):
    b = io.BytesIO()
    
    # Block content
    block = io.BytesIO()
    block.write(block_idx.to_bytes(4, 'big'))
    block.write((0).to_bytes(4, 'big'))
    block.write((0).to_bytes(4, 'big'))
    block.write((0).to_bytes(4, 'big'))
    if compressed_data is not None:
        block.write((1).to_bytes(4, 'big')) # has_data = 1
        block.write((len(compressed_data) + 4).to_bytes(4, 'big')) # subblock_len (BE)
        block.write(len(compressed_data).to_bytes(4, 'little'))    # subblock_len (LE)
        block.write(compressed_data)
    else:
        block.write((0).to_bytes(4, 'big')) # has_data = 0
    block_bytes = block.getvalue()
    
    # Wrap with BlockDataBeginChunk and BlockDataEndChunk
    # block_size is total size of this entry
    # 4 (block_size) + 4 (str len) + 38 (begin chunk) + len(block_bytes) + 4 (str len) + 34 (end chunk)
    total_size = 4 + 4 + len(BlockDataBeginChunk) + len(block_bytes) + 4 + len(BlockDataEndChunk)
    
    b.write(total_size.to_bytes(4, 'big'))
    b.write((19).to_bytes(4, 'big'))
    b.write(BlockDataBeginChunk)
    b.write(block_bytes)
    b.write((17).to_bytes(4, 'big'))
    b.write(BlockDataEndChunk)
    
    return b.getvalue()

# Test 1 non-empty tile with zlib dummy data and 1 empty tile
raw_pixels = b'\xFF' * (5 * 256 * 256) # 5 channels: 1 alpha + 4 BGRX
compressed = zlib.compress(raw_pixels)

chunk_data = bytearray()
chunk_data += create_tile_block(0, compressed)
chunk_data += create_tile_block(1, None)

blocks = parse_chunk_with_blocks(memoryview(chunk_data))
print(f"parse_chunk_with_blocks returned: {len(blocks)} blocks")
assert blocks[0] == compressed, "Block 0 data mismatch!"
assert blocks[1] is None, "Block 1 should be None!"
print("SUCCESS! Blocks match 100% with clip_to_psd parser!")
