const ONES: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];
const TENS: [&str; 10] = [
    "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

fn below_thousand(n: i64) -> String {
    if n < 20 {
        return ONES[n as usize].to_string();
    }
    if n < 100 {
        let t = TENS[(n / 10) as usize];
        let o = n % 10;
        if o == 0 {
            t.to_string()
        } else {
            format!("{t}-{}", ONES[o as usize])
        }
    } else {
        let h = n / 100;
        let rest = n % 100;
        if rest == 0 {
            format!("{} hundred", ONES[h as usize])
        } else {
            format!("{} hundred {}", ONES[h as usize], below_thousand(rest))
        }
    }
}

pub fn verbalize(n: i64) -> String {
    let sign = if n < 0 { "minus " } else { "" };
    let mut x = n.abs();
    if x == 0 {
        return "zero".to_string();
    }
    if x > 999_999_999 {
        return n.to_string();
    }
    let mut parts = Vec::new();
    let billions = x / 1_000_000_000;
    x %= 1_000_000_000;
    let millions = x / 1_000_000;
    x %= 1_000_000;
    let thousands = x / 1000;
    let rest = x % 1000;
    if billions > 0 {
        parts.push(format!("{} billion", below_thousand(billions)));
    }
    if millions > 0 {
        parts.push(format!("{} million", below_thousand(millions)));
    }
    if thousands > 0 {
        parts.push(format!("{} thousand", below_thousand(thousands)));
    }
    if rest > 0 {
        parts.push(below_thousand(rest));
    }
    format!("{sign}{}", parts.join(" "))
}

pub fn to_roman(n: i64, lower: bool) -> String {
    let x = n.abs();
    if x <= 0 || x >= 4000 {
        return n.to_string();
    }
    let map = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut left = x;
    let mut out = String::new();
    for (v, s) in map {
        while left >= v {
            out.push_str(s);
            left -= v;
        }
    }
    if lower { out.to_lowercase() } else { out }
}

pub fn format_number(n: i64, mode: &str) -> String {
    match mode {
        "verbal" => verbalize(n),
        "roman" | "roman-upper" => to_roman(n, false),
        "roman-lower" => to_roman(n, true),
        "hex" | "hex-upper" => format!("{:X}", n),
        "hex-lower" => format!("{:x}", n),
        "binary" => format!("{:b}", n),
        _ => n.to_string(),
    }
}
