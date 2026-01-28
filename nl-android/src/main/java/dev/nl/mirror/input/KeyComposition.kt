package dev.nl.mirror.input

/**
 * KeyComposition - Decompose accented characters
 * 
 * Ported from scrcpy's KeyComposition.java
 * 
 * For example, decompose('é') returns "\u0301e"
 * This is useful for injecting key events to generate accented characters
 * (KeyCharacterMap.getEvents() returns null for 'é' but works with "\u0301e")
 * 
 * See: https://source.android.com/devices/input/key-character-map-files#behaviors
 */
object KeyComposition {
    
    private const val KEY_DEAD_GRAVE = "\u0300"      // `
    private const val KEY_DEAD_ACUTE = "\u0301"      // ´
    private const val KEY_DEAD_CIRCUMFLEX = "\u0302" // ^
    private const val KEY_DEAD_TILDE = "\u0303"      // ~
    private const val KEY_DEAD_UMLAUT = "\u0308"     // ¨
    
    private val compositionMap: Map<Char, String> = buildMap {
        // Grave accent (`)
        put('À', grave('A')); put('à', grave('a'))
        put('È', grave('E')); put('è', grave('e'))
        put('Ì', grave('I')); put('ì', grave('i'))
        put('Ò', grave('O')); put('ò', grave('o'))
        put('Ù', grave('U')); put('ù', grave('u'))
        put('Ǹ', grave('N')); put('ǹ', grave('n'))
        put('Ẁ', grave('W')); put('ẁ', grave('w'))
        put('Ỳ', grave('Y')); put('ỳ', grave('y'))
        
        // Acute accent (´)
        put('Á', acute('A')); put('á', acute('a'))
        put('É', acute('E')); put('é', acute('e'))
        put('Í', acute('I')); put('í', acute('i'))
        put('Ó', acute('O')); put('ó', acute('o'))
        put('Ú', acute('U')); put('ú', acute('u'))
        put('Ý', acute('Y')); put('ý', acute('y'))
        put('Ć', acute('C')); put('ć', acute('c'))
        put('Ĺ', acute('L')); put('ĺ', acute('l'))
        put('Ń', acute('N')); put('ń', acute('n'))
        put('Ŕ', acute('R')); put('ŕ', acute('r'))
        put('Ś', acute('S')); put('ś', acute('s'))
        put('Ź', acute('Z')); put('ź', acute('z'))
        put('Ǵ', acute('G')); put('ǵ', acute('g'))
        put('Ḱ', acute('K')); put('ḱ', acute('k'))
        put('Ḿ', acute('M')); put('ḿ', acute('m'))
        put('Ṕ', acute('P')); put('ṕ', acute('p'))
        put('Ẃ', acute('W')); put('ẃ', acute('w'))
        
        // Circumflex (^)
        put('Â', circumflex('A')); put('â', circumflex('a'))
        put('Ê', circumflex('E')); put('ê', circumflex('e'))
        put('Î', circumflex('I')); put('î', circumflex('i'))
        put('Ô', circumflex('O')); put('ô', circumflex('o'))
        put('Û', circumflex('U')); put('û', circumflex('u'))
        put('Ĉ', circumflex('C')); put('ĉ', circumflex('c'))
        put('Ĝ', circumflex('G')); put('ĝ', circumflex('g'))
        put('Ĥ', circumflex('H')); put('ĥ', circumflex('h'))
        put('Ĵ', circumflex('J')); put('ĵ', circumflex('j'))
        put('Ŝ', circumflex('S')); put('ŝ', circumflex('s'))
        put('Ŵ', circumflex('W')); put('ŵ', circumflex('w'))
        put('Ŷ', circumflex('Y')); put('ŷ', circumflex('y'))
        put('Ẑ', circumflex('Z')); put('ẑ', circumflex('z'))
        
        // Tilde (~)
        put('Ã', tilde('A')); put('ã', tilde('a'))
        put('Ñ', tilde('N')); put('ñ', tilde('n'))
        put('Õ', tilde('O')); put('õ', tilde('o'))
        put('Ĩ', tilde('I')); put('ĩ', tilde('i'))
        put('Ũ', tilde('U')); put('ũ', tilde('u'))
        put('Ẽ', tilde('E')); put('ẽ', tilde('e'))
        put('Ỹ', tilde('Y')); put('ỹ', tilde('y'))
        
        // Umlaut (¨)
        put('Ä', umlaut('A')); put('ä', umlaut('a'))
        put('Ë', umlaut('E')); put('ë', umlaut('e'))
        put('Ï', umlaut('I')); put('ï', umlaut('i'))
        put('Ö', umlaut('O')); put('ö', umlaut('o'))
        put('Ü', umlaut('U')); put('ü', umlaut('u'))
        put('Ÿ', umlaut('Y')); put('ÿ', umlaut('y'))
        put('Ḧ', umlaut('H')); put('ḧ', umlaut('h'))
        put('Ẅ', umlaut('W')); put('ẅ', umlaut('w'))
        put('Ẍ', umlaut('X')); put('ẍ', umlaut('x'))
        put('ẗ', umlaut('t'))
    }
    
    private fun grave(c: Char) = KEY_DEAD_GRAVE + c
    private fun acute(c: Char) = KEY_DEAD_ACUTE + c
    private fun circumflex(c: Char) = KEY_DEAD_CIRCUMFLEX + c
    private fun tilde(c: Char) = KEY_DEAD_TILDE + c
    private fun umlaut(c: Char) = KEY_DEAD_UMLAUT + c
    
    /**
     * Decompose an accented character into a combining character followed by the base character.
     * @param c The character to decompose
     * @return The decomposed string, or null if the character is not a known accented character
     */
    fun decompose(c: Char): String? = compositionMap[c]
}
