import zlib; 

bt = bytes([120, 156, 74, 173, 72, 204, 45, 200, 73, 5, 0, 0, 0, 255, 255, 3, 0, 11, 192, 2, 237])
str = zlib.decompress(bt)
print(str)