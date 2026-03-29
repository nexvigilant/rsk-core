//! DNA codec for heligrams.
//!
//! Bijective byte↔nucleotide encoding (nucli algorithm):
//! A=0b00, T=0b01, G=0b10, C=0b11. Each byte → 4 nucleotides.
//! 4^4 = 256 = byte range. Perfect bijection.
//!
//! A heligram YAML file encodes to a DNA sequence that IS the program.
//! The antisense strand is the reverse complement — computable from the sense strand
//! via Watson-Crick base pairing (A↔T, G↔C).

/// Encode bytes to DNA strand. Each byte → 4 nucleotides (MSB first).
pub fn encode(data: &[u8]) -> String {
    let mut strand = String::with_capacity(data.len() * 4);
    for &byte in data {
        strand.push(bits_to_nuc((byte >> 6) & 0b11));
        strand.push(bits_to_nuc((byte >> 4) & 0b11));
        strand.push(bits_to_nuc((byte >> 2) & 0b11));
        strand.push(bits_to_nuc(byte & 0b11));
    }
    strand
}

/// Decode DNA strand back to bytes. Strand length must be divisible by 4.
pub fn decode(strand: &str) -> Result<Vec<u8>, String> {
    if !strand.len().is_multiple_of(4) {
        return Err(format!(
            "strand length {} not divisible by 4",
            strand.len()
        ));
    }
    let chars: Vec<char> = strand.chars().collect();
    let mut bytes = Vec::with_capacity(chars.len() / 4);
    for chunk in chars.chunks(4) {
        let b0 = nuc_to_bits(chunk[0])?;
        let b1 = nuc_to_bits(chunk[1])?;
        let b2 = nuc_to_bits(chunk[2])?;
        let b3 = nuc_to_bits(chunk[3])?;
        bytes.push((b0 << 6) | (b1 << 4) | (b2 << 2) | b3);
    }
    Ok(bytes)
}

/// Reverse complement: reverse strand and swap A↔T, G↔C.
/// This is an involution: complement(complement(s)) == s.
pub fn complement(strand: &str) -> String {
    strand
        .chars()
        .rev()
        .map(|c| match c {
            'A' => 'T',
            'T' => 'A',
            'G' => 'C',
            'C' => 'G',
            other => other,
        })
        .collect()
}

const fn bits_to_nuc(bits: u8) -> char {
    match bits & 0b11 {
        0b01 => 'T',
        0b10 => 'G',
        0b11 => 'C',
        _ => 'A',
    }
}

fn nuc_to_bits(ch: char) -> Result<u8, String> {
    match ch {
        'A' | 'a' => Ok(0b00),
        'T' | 't' => Ok(0b01),
        'G' | 'g' => Ok(0b10),
        'C' | 'c' => Ok(0b11),
        other => Err(format!("invalid nucleotide: {other}")),
    }
}

/// Encode a heligram YAML file to DNA. Returns (sense_dna, antisense_dna, stats).
pub fn encode_heligram(yaml_bytes: &[u8]) -> HeligramDna {
    let sense = encode(yaml_bytes);
    let antisense = complement(&sense);
    let nucleotides = sense.len();
    let codons = nucleotides / 3; // reading frame codons
    HeligramDna {
        sense,
        antisense,
        nucleotides,
        codons,
        bytes: yaml_bytes.len(),
    }
}

/// DNA-encoded heligram with both strands.
#[derive(Debug, Clone)]
pub struct HeligramDna {
    /// Sense strand (5'→3') — the program encoding.
    pub sense: String,
    /// Antisense strand (3'→5') — reverse complement.
    pub antisense: String,
    /// Total nucleotides in sense strand.
    pub nucleotides: usize,
    /// Codons (nucleotides / 3, reading frame).
    pub codons: usize,
    /// Original byte count.
    pub bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_byte() {
        for b in 0..=255u8 {
            let dna = encode(&[b]);
            let decoded = decode(&dna).unwrap_or_default();
            assert_eq!(decoded, vec![b], "failed roundtrip for byte {b}");
        }
    }

    #[test]
    fn roundtrip_string() {
        let msg = b"prr-signal-helix";
        let dna = encode(msg);
        let decoded = decode(&dna).unwrap_or_default();
        assert_eq!(&decoded, msg);
    }

    #[test]
    fn complement_is_involution() {
        let dna = encode(b"HELIGRAM");
        let comp = complement(&dna);
        let comp_comp = complement(&comp);
        assert_eq!(dna, comp_comp);
    }

    #[test]
    fn complement_swaps_correctly() {
        assert_eq!(complement("ATGC"), "GCAT");
    }

    #[test]
    fn encode_heligram_produces_both_strands() {
        let yaml = b"name: test\ntype: heligram\n";
        let result = encode_heligram(yaml);
        assert_eq!(result.bytes, yaml.len());
        assert_eq!(result.nucleotides, yaml.len() * 4);
        assert_eq!(complement(&result.sense), result.antisense);
    }

    #[test]
    fn decode_rejects_bad_length() {
        assert!(decode("ATG").is_err());
    }

    #[test]
    fn decode_rejects_bad_char() {
        assert!(decode("ATGX").is_err());
    }
}
