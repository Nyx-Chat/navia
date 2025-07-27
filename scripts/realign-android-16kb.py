#!/usr/bin/env python3
"""
Realign Android .so file LOAD segments to 16KB boundaries.
This is required for Android 15+ compatibility.
"""

import sys
import os
import struct
import shutil

def align_to_16kb(value):
    """Align a value up to the next 16KB boundary."""
    alignment = 16 * 1024  # 16KB
    return ((value + alignment - 1) // alignment) * alignment

def read_elf_header(f):
    """Read and validate ELF header."""
    f.seek(0)
    elf_header = f.read(64)
    
    # Verify it's an ELF file
    if elf_header[:4] != b'\x7fELF':
        raise ValueError("Not an ELF file")
    
    # Check if 32 or 64 bit
    ei_class = elf_header[4]
    is_64bit = (ei_class == 2)
    
    if is_64bit:
        # 64-bit ELF
        e_phoff = struct.unpack('<Q', elf_header[32:40])[0]
        e_phentsize = struct.unpack('<H', elf_header[54:56])[0]
        e_phnum = struct.unpack('<H', elf_header[56:58])[0]
    else:
        # 32-bit ELF
        e_phoff = struct.unpack('<I', elf_header[28:32])[0]
        e_phentsize = struct.unpack('<H', elf_header[42:44])[0]
        e_phnum = struct.unpack('<H', elf_header[44:46])[0]
    
    return is_64bit, e_phoff, e_phentsize, e_phnum

def realign_elf(input_file, output_file):
    """Realign ELF file to ensure 16KB alignment of LOAD segments."""
    
    # Read the entire file into memory
    with open(input_file, 'rb') as f:
        original_data = f.read()
    
    # We'll build a new file with proper alignment
    data = bytearray(original_data)
    
    # Parse ELF header
    with open(input_file, 'rb') as f:
        is_64bit, e_phoff, e_phentsize, e_phnum = read_elf_header(f)
    
    print(f"Processing {'64' if is_64bit else '32'}-bit ELF file")
    print(f"Program headers: offset=0x{e_phoff:x}, size={e_phentsize}, count={e_phnum}")
    
    # Track if we made any changes
    modified = False
    
    # First pass: collect all LOAD segments and calculate new offsets
    load_segments = []
    
    for i in range(e_phnum):
        ph_offset = e_phoff + (i * e_phentsize)
        
        if is_64bit:
            # 64-bit program header
            p_type = struct.unpack_from('<I', data, ph_offset)[0]
            
            if p_type == 1:  # PT_LOAD
                p_offset_off = ph_offset + 8
                p_vaddr_off = ph_offset + 16
                p_paddr_off = ph_offset + 24
                p_filesz_off = ph_offset + 32
                p_memsz_off = ph_offset + 40
                p_align_off = ph_offset + 48
                
                p_offset = struct.unpack_from('<Q', data, p_offset_off)[0]
                p_vaddr = struct.unpack_from('<Q', data, p_vaddr_off)[0]
                p_paddr = struct.unpack_from('<Q', data, p_paddr_off)[0]
                p_filesz = struct.unpack_from('<Q', data, p_filesz_off)[0]
                p_memsz = struct.unpack_from('<Q', data, p_memsz_off)[0]
                p_align = struct.unpack_from('<Q', data, p_align_off)[0]
                
                load_segments.append({
                    'index': i,
                    'offset': p_offset,
                    'vaddr': p_vaddr,
                    'paddr': p_paddr,
                    'filesz': p_filesz,
                    'memsz': p_memsz,
                    'align': p_align,
                    'offset_off': p_offset_off,
                    'vaddr_off': p_vaddr_off,
                    'paddr_off': p_paddr_off,
                    'align_off': p_align_off
                })
    
    # Second pass: update addresses to be 16KB aligned
    for seg in load_segments:
        # Check if alignment is needed for virtual address
        if seg['vaddr'] % 16384 != 0:
            old_vaddr = seg['vaddr']
            new_vaddr = align_to_16kb(seg['vaddr'])
            
            print(f"LOAD segment {seg['index']}: VirtAddr 0x{old_vaddr:x} -> 0x{new_vaddr:x}")
            
            # Update virtual and physical addresses
            struct.pack_into('<Q', data, seg['vaddr_off'], new_vaddr)
            struct.pack_into('<Q', data, seg['paddr_off'], new_vaddr)
            modified = True
        
        # Ensure alignment is set to 16KB
        if seg['align'] < 16384:
            struct.pack_into('<Q', data, seg['align_off'], 16384)
            modified = True
        else:
            # 32-bit program header
            p_type = struct.unpack_from('<I', data, ph_offset)[0]
            
            if p_type == 1:  # PT_LOAD
                p_offset_off = ph_offset + 4
                p_vaddr_off = ph_offset + 8
                p_paddr_off = ph_offset + 12
                p_align_off = ph_offset + 28
                
                p_offset = struct.unpack_from('<I', data, p_offset_off)[0]
                p_vaddr = struct.unpack_from('<I', data, p_vaddr_off)[0]
                p_paddr = struct.unpack_from('<I', data, p_paddr_off)[0]
                p_align = struct.unpack_from('<I', data, p_align_off)[0]
                
                # Check if alignment is needed
                if p_vaddr % 16384 != 0:
                    old_vaddr = p_vaddr
                    new_vaddr = align_to_16kb(p_vaddr)
                    
                    print(f"LOAD segment {i}: VirtAddr 0x{old_vaddr:x} -> 0x{new_vaddr:x}")
                    
                    # Update virtual and physical addresses
                    struct.pack_into('<I', data, p_vaddr_off, new_vaddr)
                    struct.pack_into('<I', data, p_paddr_off, new_vaddr)
                    modified = True
                
                # Ensure alignment is set to 16KB
                if p_align < 16384:
                    struct.pack_into('<I', data, p_align_off, 16384)
                    modified = True
    
    if modified:
        # Write the modified file
        with open(output_file, 'wb') as f:
            f.write(data)
        print(f"Modified file written to: {output_file}")
        return True
    else:
        print("No modifications needed - file already aligned")
        shutil.copy2(input_file, output_file)
        return False

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <input.so> <output.so>")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = sys.argv[2]
    
    if not os.path.exists(input_file):
        print(f"Error: {input_file} not found")
        sys.exit(1)
    
    print(f"Realigning: {input_file}")
    try:
        realign_elf(input_file, output_file)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)