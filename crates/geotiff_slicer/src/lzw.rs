/// TIFF LZW decoder (ported from Dart GeoTiffLzwDecoder).
pub struct GeoTiffLzwDecoder {
    bits_to_get: usize,
    byte_pointer: usize,
    next_data: u32,
    next_bits: usize,
    data_length: usize,
    out: *mut u8,
    out_len: usize,
    out_pointer: usize,
    buffer: [u8; 4096],
    table: Vec<u8>,
    prefix: Vec<u32>,
    table_index: usize,
    buffer_length: usize,
}

const LZ_MAX_CODE: usize = 4095;
const NO_SUCH_CODE: u32 = 4098;
const AND_TABLE: [u32; 4] = [511, 1023, 2047, 4095];

impl GeoTiffLzwDecoder {
    pub fn new() -> Self {
        Self {
            bits_to_get: 9,
            byte_pointer: 0,
            next_data: 0,
            next_bits: 0,
            data_length: 0,
            out: std::ptr::null_mut(),
            out_len: 0,
            out_pointer: 0,
            buffer: [0; 4096],
            table: Vec::new(),
            prefix: Vec::new(),
            table_index: 258,
            buffer_length: 0,
        }
    }

    pub fn decode(
        &mut self,
        input: &[u8],
        input_offset: usize,
        input_length: usize,
        output: &mut [u8],
    ) {
        self.byte_pointer = input_offset;
        self.data_length = input_offset + input_length;
        self.out = output.as_mut_ptr();
        self.out_len = output.len();
        self.out_pointer = 0;

        if self.byte_pointer + 1 < self.data_length
            && input[self.byte_pointer] == 0x00
            && input[self.byte_pointer + 1] == 0x01
        {
            panic!("Некорректные LZW-данные TIFF");
        }

        self.initialize_string_table();
        self.next_data = 0;
        self.next_bits = 0;

        let mut old_code = 0u32;
        let mut code = self.get_next_code(input);

        while code != 257 && self.out_pointer < self.out_len {
            if code == 256 {
                self.initialize_string_table();
                code = self.get_next_code(input);
                self.buffer_length = 0;
                if code == 257 {
                    break;
                }
                self.write_out(code as u8);
                old_code = code;
            } else if (code as usize) < self.table_index {
                self.get_string(code as usize);
                self.write_buffer_rev();
                self.add_string(old_code as usize, self.buffer[self.buffer_length - 1]);
                old_code = code;
            } else {
                self.get_string(old_code as usize);
                self.write_buffer_rev();
                self.write_out(self.buffer[self.buffer_length - 1]);
                self.add_string(old_code as usize, self.buffer[self.buffer_length - 1]);
                old_code = code;
            }
            code = self.get_next_code(input);
        }
    }

    fn write_out(&mut self, byte: u8) {
        if self.out_pointer < self.out_len {
            unsafe {
                *self.out.add(self.out_pointer) = byte;
            }
            self.out_pointer += 1;
        }
    }

    fn write_buffer_rev(&mut self) {
        let len = self.buffer_length;
        let available = self.out_len - self.out_pointer;
        let n = len.min(available);
        unsafe {
            let out = self.out.add(self.out_pointer);
            for i in 0..n {
                *out.add(i) = self.buffer[len - 1 - i];
            }
        }
        self.out_pointer += n;
    }

    fn add_string(&mut self, string: usize, new_string: u8) {
        self.table[self.table_index] = new_string;
        self.prefix[self.table_index] = string as u32;
        self.table_index += 1;

        if self.table_index == 511 {
            self.bits_to_get = 10;
        } else if self.table_index == 1023 {
            self.bits_to_get = 11;
        } else if self.table_index == 2047 {
            self.bits_to_get = 12;
        }
    }

    fn get_string(&mut self, code: usize) {
        self.buffer_length = 0;
        let mut c = code;
        self.buffer[self.buffer_length] = self.table[c];
        self.buffer_length += 1;
        c = self.prefix[c] as usize;
        while c != NO_SUCH_CODE as usize {
            self.buffer[self.buffer_length] = self.table[c];
            self.buffer_length += 1;
            c = self.prefix[c] as usize;
        }
    }

    fn get_next_code(&mut self, input: &[u8]) -> u32 {
        if self.byte_pointer >= self.data_length {
            return 257;
        }

        while self.next_bits < self.bits_to_get {
            if self.byte_pointer >= self.data_length {
                return 257;
            }
            self.next_data =
                ((self.next_data << 8) + input[self.byte_pointer] as u32) & 0xffff_ffff;
            self.byte_pointer += 1;
            self.next_bits += 8;
        }

        self.next_bits -= self.bits_to_get;
        (self.next_data >> self.next_bits) & AND_TABLE[self.bits_to_get - 9]
    }

    fn initialize_string_table(&mut self) {
        if self.table.is_empty() {
            self.table = vec![0u8; LZ_MAX_CODE + 1];
            self.prefix = vec![NO_SUCH_CODE; LZ_MAX_CODE + 1];
        } else {
            self.prefix.fill(NO_SUCH_CODE);
        }
        for i in 0..256 {
            self.table[i] = i as u8;
        }
        self.bits_to_get = 9;
        self.table_index = 258;
    }
}

impl Default for GeoTiffLzwDecoder {
    fn default() -> Self {
        Self::new()
    }
}

// Сырой указатель на output живёт только на время decode(); между вызовами не используется.
// Нужен Send, чтобы rayon мог держать воркер с LZW в thread-local state.
unsafe impl Send for GeoTiffLzwDecoder {}
