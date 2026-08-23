export const IPA_SECTIONS = ["consonants", "vowels", "modifiers", "suprasegmentals"] as const;

export type IpaSection = (typeof IPA_SECTIONS)[number];

export type IpaSymbol = {
  symbol: string;
  name: string;
  section: IpaSection;
  group: string;
  display?: string;
};

export const IPA_SECTION_LABELS: Record<IpaSection, string> = {
  consonants: "Consonants",
  vowels: "Vowels",
  modifiers: "Diacritics & modifiers",
  suprasegmentals: "Suprasegmentals",
};

export const IPA_SYMBOLS: IpaSymbol[] = [
  { symbol: "p", name: "Voiceless bilabial plosive", section: "consonants", group: "Plosives" },
  { symbol: "b", name: "Voiced bilabial plosive", section: "consonants", group: "Plosives" },
  { symbol: "t", name: "Voiceless alveolar plosive", section: "consonants", group: "Plosives" },
  { symbol: "d", name: "Voiced alveolar plosive", section: "consonants", group: "Plosives" },
  { symbol: "ʈ", name: "Voiceless retroflex plosive", section: "consonants", group: "Plosives" },
  { symbol: "ɖ", name: "Voiced retroflex plosive", section: "consonants", group: "Plosives" },
  { symbol: "c", name: "Voiceless palatal plosive", section: "consonants", group: "Plosives" },
  { symbol: "ɟ", name: "Voiced palatal plosive", section: "consonants", group: "Plosives" },
  { symbol: "k", name: "Voiceless velar plosive", section: "consonants", group: "Plosives" },
  { symbol: "ɡ", name: "Voiced velar plosive", section: "consonants", group: "Plosives" },
  { symbol: "q", name: "Voiceless uvular plosive", section: "consonants", group: "Plosives" },
  { symbol: "ɢ", name: "Voiced uvular plosive", section: "consonants", group: "Plosives" },
  { symbol: "ʔ", name: "Glottal plosive", section: "consonants", group: "Plosives" },
  { symbol: "m", name: "Bilabial nasal", section: "consonants", group: "Nasals" },
  { symbol: "ɱ", name: "Labiodental nasal", section: "consonants", group: "Nasals" },
  { symbol: "n", name: "Alveolar nasal", section: "consonants", group: "Nasals" },
  { symbol: "ɳ", name: "Retroflex nasal", section: "consonants", group: "Nasals" },
  { symbol: "ɲ", name: "Palatal nasal", section: "consonants", group: "Nasals" },
  { symbol: "ŋ", name: "Velar nasal", section: "consonants", group: "Nasals" },
  { symbol: "ɴ", name: "Uvular nasal", section: "consonants", group: "Nasals" },
  { symbol: "ʙ", name: "Bilabial trill", section: "consonants", group: "Trills and taps" },
  { symbol: "r", name: "Alveolar trill", section: "consonants", group: "Trills and taps" },
  { symbol: "ʀ", name: "Uvular trill", section: "consonants", group: "Trills and taps" },
  { symbol: "ɾ", name: "Alveolar tap or flap", section: "consonants", group: "Trills and taps" },
  { symbol: "ɽ", name: "Retroflex flap", section: "consonants", group: "Trills and taps" },
  { symbol: "ɸ", name: "Voiceless bilabial fricative", section: "consonants", group: "Fricatives" },
  { symbol: "β", name: "Voiced bilabial fricative", section: "consonants", group: "Fricatives" },
  { symbol: "f", name: "Voiceless labiodental fricative", section: "consonants", group: "Fricatives" },
  { symbol: "v", name: "Voiced labiodental fricative", section: "consonants", group: "Fricatives" },
  { symbol: "θ", name: "Voiceless dental fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ð", name: "Voiced dental fricative", section: "consonants", group: "Fricatives" },
  { symbol: "s", name: "Voiceless alveolar fricative", section: "consonants", group: "Fricatives" },
  { symbol: "z", name: "Voiced alveolar fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʃ", name: "Voiceless postalveolar fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʒ", name: "Voiced postalveolar fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʂ", name: "Voiceless retroflex fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʐ", name: "Voiced retroflex fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ç", name: "Voiceless palatal fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʝ", name: "Voiced palatal fricative", section: "consonants", group: "Fricatives" },
  { symbol: "x", name: "Voiceless velar fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ɣ", name: "Voiced velar fricative", section: "consonants", group: "Fricatives" },
  { symbol: "χ", name: "Voiceless uvular fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʁ", name: "Voiced uvular fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ħ", name: "Voiceless pharyngeal fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ʕ", name: "Voiced pharyngeal fricative", section: "consonants", group: "Fricatives" },
  { symbol: "h", name: "Voiceless glottal fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ɦ", name: "Voiced glottal fricative", section: "consonants", group: "Fricatives" },
  { symbol: "ɬ", name: "Voiceless alveolar lateral fricative", section: "consonants", group: "Lateral fricatives" },
  { symbol: "ɮ", name: "Voiced alveolar lateral fricative", section: "consonants", group: "Lateral fricatives" },
  { symbol: "j", name: "Palatal approximant", section: "consonants", group: "Approximants" },
  { symbol: "w", name: "Voiced labial-velar approximant", section: "consonants", group: "Approximants" },
  { symbol: "ɹ", name: "Alveolar approximant", section: "consonants", group: "Approximants" },
  { symbol: "ɻ", name: "Retroflex approximant", section: "consonants", group: "Approximants" },
  { symbol: "l", name: "Alveolar lateral approximant", section: "consonants", group: "Lateral approximants" },
  { symbol: "ɭ", name: "Retroflex lateral approximant", section: "consonants", group: "Lateral approximants" },
  { symbol: "ʎ", name: "Palatal lateral approximant", section: "consonants", group: "Lateral approximants" },
  { symbol: "ʟ", name: "Velar lateral approximant", section: "consonants", group: "Lateral approximants" },
  { symbol: "t͡s", name: "Voiceless alveolar affricate", section: "consonants", group: "Common affricates" },
  { symbol: "d͡z", name: "Voiced alveolar affricate", section: "consonants", group: "Common affricates" },
  { symbol: "t͡ʃ", name: "Voiceless postalveolar affricate", section: "consonants", group: "Common affricates" },
  { symbol: "d͡ʒ", name: "Voiced postalveolar affricate", section: "consonants", group: "Common affricates" },

  { symbol: "i", name: "Close front unrounded vowel", section: "vowels", group: "Front" },
  { symbol: "y", name: "Close front rounded vowel", section: "vowels", group: "Front" },
  { symbol: "ɪ", name: "Near-close near-front unrounded vowel", section: "vowels", group: "Front" },
  { symbol: "ʏ", name: "Near-close near-front rounded vowel", section: "vowels", group: "Front" },
  { symbol: "e", name: "Close-mid front unrounded vowel", section: "vowels", group: "Front" },
  { symbol: "ø", name: "Close-mid front rounded vowel", section: "vowels", group: "Front" },
  { symbol: "ɛ", name: "Open-mid front unrounded vowel", section: "vowels", group: "Front" },
  { symbol: "œ", name: "Open-mid front rounded vowel", section: "vowels", group: "Front" },
  { symbol: "æ", name: "Near-open front unrounded vowel", section: "vowels", group: "Front" },
  { symbol: "a", name: "Open front unrounded vowel", section: "vowels", group: "Front" },
  { symbol: "ɨ", name: "Close central unrounded vowel", section: "vowels", group: "Central" },
  { symbol: "ʉ", name: "Close central rounded vowel", section: "vowels", group: "Central" },
  { symbol: "ɘ", name: "Close-mid central unrounded vowel", section: "vowels", group: "Central" },
  { symbol: "ɵ", name: "Close-mid central rounded vowel", section: "vowels", group: "Central" },
  { symbol: "ə", name: "Mid central vowel", section: "vowels", group: "Central" },
  { symbol: "ɜ", name: "Open-mid central unrounded vowel", section: "vowels", group: "Central" },
  { symbol: "ɞ", name: "Open-mid central rounded vowel", section: "vowels", group: "Central" },
  { symbol: "ɐ", name: "Near-open central vowel", section: "vowels", group: "Central" },
  { symbol: "ɯ", name: "Close back unrounded vowel", section: "vowels", group: "Back" },
  { symbol: "u", name: "Close back rounded vowel", section: "vowels", group: "Back" },
  { symbol: "ʊ", name: "Near-close near-back rounded vowel", section: "vowels", group: "Back" },
  { symbol: "ɤ", name: "Close-mid back unrounded vowel", section: "vowels", group: "Back" },
  { symbol: "o", name: "Close-mid back rounded vowel", section: "vowels", group: "Back" },
  { symbol: "ʌ", name: "Open-mid back unrounded vowel", section: "vowels", group: "Back" },
  { symbol: "ɔ", name: "Open-mid back rounded vowel", section: "vowels", group: "Back" },
  { symbol: "ɑ", name: "Open back unrounded vowel", section: "vowels", group: "Back" },
  { symbol: "ɒ", name: "Open back rounded vowel", section: "vowels", group: "Back" },

  { symbol: "ʰ", name: "Aspirated", section: "modifiers", group: "Release" },
  { symbol: "ⁿ", name: "Nasal release", section: "modifiers", group: "Release" },
  { symbol: "ˡ", name: "Lateral release", section: "modifiers", group: "Release" },
  { symbol: "̚", display: "◌̚", name: "No audible release", section: "modifiers", group: "Release" },
  { symbol: "̃", display: "◌̃", name: "Nasalized", section: "modifiers", group: "Voice and airflow" },
  { symbol: "̥", display: "◌̥", name: "Voiceless or devoiced", section: "modifiers", group: "Voice and airflow" },
  { symbol: "̬", display: "◌̬", name: "Voiced", section: "modifiers", group: "Voice and airflow" },
  { symbol: "̤", display: "◌̤", name: "Breathy voiced", section: "modifiers", group: "Voice and airflow" },
  { symbol: "̰", display: "◌̰", name: "Creaky voiced", section: "modifiers", group: "Voice and airflow" },
  { symbol: "ʲ", name: "Palatalized", section: "modifiers", group: "Secondary articulation" },
  { symbol: "ʷ", name: "Labialized", section: "modifiers", group: "Secondary articulation" },
  { symbol: "ˠ", name: "Velarized", section: "modifiers", group: "Secondary articulation" },
  { symbol: "ˤ", name: "Pharyngealized", section: "modifiers", group: "Secondary articulation" },
  { symbol: "̩", display: "◌̩", name: "Syllabic", section: "modifiers", group: "Syllabicity" },
  { symbol: "̯", display: "◌̯", name: "Non-syllabic", section: "modifiers", group: "Syllabicity" },
  { symbol: "̪", display: "◌̪", name: "Dental", section: "modifiers", group: "Articulation" },
  { symbol: "̺", display: "◌̺", name: "Apical", section: "modifiers", group: "Articulation" },
  { symbol: "̻", display: "◌̻", name: "Laminal", section: "modifiers", group: "Articulation" },

  { symbol: "ˈ", name: "Primary stress", section: "suprasegmentals", group: "Stress and boundaries" },
  { symbol: "ˌ", name: "Secondary stress", section: "suprasegmentals", group: "Stress and boundaries" },
  { symbol: "ː", name: "Long", section: "suprasegmentals", group: "Length" },
  { symbol: "ˑ", name: "Half-long", section: "suprasegmentals", group: "Length" },
  { symbol: ".", name: "Syllable boundary", section: "suprasegmentals", group: "Stress and boundaries" },
  { symbol: "‿", name: "Linking or absence of a break", section: "suprasegmentals", group: "Stress and boundaries" },
  { symbol: "|", name: "Minor prosodic break", section: "suprasegmentals", group: "Stress and boundaries" },
  { symbol: "‖", name: "Major prosodic break", section: "suprasegmentals", group: "Stress and boundaries" },
  { symbol: "˥", name: "Extra-high tone", section: "suprasegmentals", group: "Tone" },
  { symbol: "˦", name: "High tone", section: "suprasegmentals", group: "Tone" },
  { symbol: "˧", name: "Mid tone", section: "suprasegmentals", group: "Tone" },
  { symbol: "˨", name: "Low tone", section: "suprasegmentals", group: "Tone" },
  { symbol: "˩", name: "Extra-low tone", section: "suprasegmentals", group: "Tone" },
  { symbol: "↗", name: "Global rise", section: "suprasegmentals", group: "Tone" },
  { symbol: "↘", name: "Global fall", section: "suprasegmentals", group: "Tone" },
];

export function searchIpaSymbols(query: string, symbols: IpaSymbol[] = IPA_SYMBOLS): IpaSymbol[] {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return symbols;
  return symbols.filter((entry) => {
    const searchable =
      `${entry.symbol} ${entry.name} ${entry.group} ${IPA_SECTION_LABELS[entry.section]}`.toLocaleLowerCase();
    return terms.every((term) => searchable.includes(term));
  });
}

export function insertIpaAtSelection(value: string, symbol: string, selectionStart: number, selectionEnd: number) {
  const start = Math.max(0, Math.min(selectionStart, value.length));
  const end = Math.max(start, Math.min(selectionEnd, value.length));
  return {
    value: `${value.slice(0, start)}${symbol}${value.slice(end)}`,
    cursor: start + symbol.length,
  };
}
