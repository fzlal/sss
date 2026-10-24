use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Persian,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::English, Language::Persian];

    pub fn label(self) -> &'static str {
        match self {
            Language::English => "ENGLISH",
            Language::Persian => "PERSIAN",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Language::English => Language::Persian,
            Language::Persian => Language::English,
        }
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(Language::English as u8);

pub fn set_language(language: Language) {
    LANGUAGE.store(language as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    match LANGUAGE.load(Ordering::Relaxed) {
        1 => Language::Persian,
        _ => Language::English,
    }
}

pub fn is_persian() -> bool {
    language() == Language::Persian
}

/// Translate every line of a logical English string. The result is still in
/// logical order; call [`display_line`] before handing it to the TTF renderer.
pub fn localize(text: &str) -> String {
    localize_for(language(), text)
}

pub fn localize_for(lang: Language, text: &str) -> String {
    if lang == Language::English {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&localize_line(line));
    }
    out
}

/// Shape Persian and put it into visual (left-to-right) order for macroquad's
/// single-direction TTF renderer.
pub fn display_line(text: &str) -> String {
    display_line_for(language(), text)
}

pub fn display_line_for(lang: Language, text: &str) -> String {
    if lang == Language::English {
        return text.to_owned();
    }
    let shaped = shape_arabic(text);
    reorder_bidi(&shaped)
}

fn dictionary() -> &'static HashMap<String, &'static str> {
    static MAP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        PHRASES
            .iter()
            .map(|(en, fa)| (en.to_ascii_lowercase(), *fa))
            .collect()
    })
}

fn exact_phrase(text: &str) -> Option<&'static str> {
    let key = text.trim().to_ascii_lowercase();
    dictionary().get(&key).copied()
}

fn localize_line(line: &str) -> String {
    if line.trim().is_empty() {
        return line.to_owned();
    }
    if let Some(fa) = exact_phrase(line) {
        return fa.to_owned();
    }
    if let Some(fa) = apply_pattern(line) {
        return fa;
    }
    if let Some(prefix) = line.trim_end().strip_suffix('!') {
        if let Some(fa) = exact_phrase(prefix) {
            return format!("{fa}!");
        }
    }
    line.to_owned()
}

fn apply_pattern(text: &str) -> Option<String> {
    let trimmed = text.trim();
    for (english, persian) in PATTERNS {
        if let Some(values) = capture(english, trimmed) {
            let translated: Vec<String> = values
                .iter()
                .map(|value| {
                    let value = value.trim();
                    if value.is_empty() {
                        String::new()
                    } else {
                        exact_phrase(value)
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_owned())
                    }
                })
                .collect();
            return Some(fill(persian, &translated));
        }
    }
    None
}

/// Matches a template made of literal chunks separated by `{}` wildcards.
fn capture<'a>(template: &str, text: &'a str) -> Option<Vec<&'a str>> {
    let parts: Vec<&str> = template.split("{}").collect();
    if parts.len() < 2 {
        return None;
    }
    if !text.starts_with(parts[0]) {
        return None;
    }
    let mut position = parts[0].len();
    let mut values = Vec::with_capacity(parts.len() - 1);
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index + 1 == parts.len() {
            if !text[position..].ends_with(part) {
                return None;
            }
            let end = text.len() - part.len();
            if end < position {
                return None;
            }
            values.push(&text[position..end]);
        } else {
            let offset = text[position..].find(part)?;
            let end = position + offset;
            values.push(&text[position..end]);
            position = end + part.len();
        }
    }
    Some(values)
}

fn fill(template: &str, values: &[String]) -> String {
    let mut out = String::new();
    for (index, part) in template.split("{}").enumerate() {
        if index > 0 {
            if let Some(value) = values.get(index - 1) {
                out.push_str(value);
            }
        }
        out.push_str(part);
    }
    out
}

const PATTERNS: &[(&str, &str)] = &[
    ("PING {} MS  ·  LOSS {}%", "پینگ {} میلی‌ثانیه  ·  گمشده {}٪"),
    ("PING {} MS", "پینگ {} میلی‌ثانیه"),
    (
        "{}  ·  {}  ·  FIRST TO {}",
        "{}  ·  {}  ·  اولین نفر تا {} امتیاز",
    ),
    ("{} / {} PLAYERS  ·  {}", "{} از {} بازیکن  ·  {}"),
    ("{} / {} PLAYERS", "{} از {} بازیکن"),
    ("CONTACTING THE HOST{}", "در حال تماس با میزبان{}"),
    ("YOU ARE {}", "شما {} هستید"),
    ("+{} MORE", "+{} بیشتر"),
    ("CUP RUN  ·  ROUND {} OF {}", "مسیر جام  ·  دور {} از {}"),
    (
        "MATCHES {}   WINS {}   CUPS {}   GOALS {}",
        "بازی‌ها {}   بردها {}   جام‌ها {}   گل‌ها {}",
    ),
    ("SPD {}  JMP {}  PWR {}", "سرعت {}  پرش {}  قدرت {}"),
    ("{}   {}  -  {}   {}", "{}   {}  -  {}   {}"),
    ("{}  ·  {}  ·  {} MANA", "{}  ·  {}  ·  {} مانا"),
    ("{}  ·  {}  ·  {}s", "{}  ·  {}  ·  {} ثانیه"),
    ("{}  ·  {}", "{}  ·  {}"),
    ("CUPS WON: {}", "جام‌های فتح‌شده: {}"),
    ("{} CUP", "جام {}"),
    ("{} GOALS", "{} امتیاز"),
    ("P{}  {}", "بازیکن {}  {}"),
    ("FIRST TO {}", "اولین نفر تا {} امتیاز"),
    ("{} SCORES", "{} امتیاز گرفت"),
    ("{} OWN GOAL", "{} گل به خودی زد"),
    ("{} NEEDS {}", "{} نیاز به {} دارد"),
    ("{} {}s", "{}  {} ثانیه"),
    ("{} FPS", "{} فریم بر ثانیه"),
    ("{}  {}", "{}  {}"),
];

const PHRASES: &[(&str, &str)] = &[
    // Menus and shared labels.
    ("QUICK MATCH", "بازی سریع"),
    ("CHAMPIONSHIP", "قهرمانی"),
    ("SETTINGS", "تنظیمات"),
    ("HOW TO PLAY", "راهنمای بازی"),
    ("QUIT", "خروج"),
    ("BACK", "بازگشت"),
    ("MATCH SETUP", "تنظیم بازی"),
    ("CHAMPIONSHIP SETUP", "تنظیم قهرمانی"),
    ("THREE WINS. ONE CUP.", "سه برد. یک جام."),
    ("PICK YOUR SLIME AND RULES", "اسلایمت و قانون‌ها را انتخاب کن"),
    ("PLAYER 1", "بازیکن 1"),
    ("PLAYER 2", "بازیکن 2"),
    ("P1", "بازیکن 1"),
    ("P2", "بازیکن 2"),
    ("PLAYER 2 / CPU", "بازیکن 2 / کامپیوتر"),
    ("CPU RIVAL", "حریف کامپیوتری"),
    ("CPU BRACKET", "جدول کامپیوتری"),
    ("RANDOM RIVALS", "حریفان تصادفی"),
    ("LOCAL 2P", "دو نفره محلی"),
    ("CPU", "کامپیوتر"),
    ("SPORT", "ورزش"),
    ("ARENA", "زمین"),
    ("MANA MODE", "حالت مانا"),
    ("PLAY TO", "امتیاز هدف"),
    ("DIFFICULTY", "سختی"),
    ("OPPONENT", "حریف"),
    ("FORMAT", "قالب"),
    ("3 ROUND CUP", "جام 3 دوره"),
    ("SINGLE MATCH", "تک‌بازی"),
    ("CHANGE", "تغییر"),
    ("START CUP", "شروع جام"),
    ("START MATCH", "شروع بازی"),
    ("DRAWN", "قرعه"),
    ("BEAT ALL THREE", "هر سه را شکست بده"),
    ("CLICK TO CHANGE", "برای تغییر کلیک کن"),
    ("SLIME READY", "اسلایم آماده"),
    ("CHOOSE SLIME", "انتخاب اسلایم"),
    ("CHOOSE ARENA", "انتخاب زمین"),
    ("ELEVEN PLACES TO SETTLE IT", "یازده زمین برای تسویه‌حساب"),
    ("AUDIO, SCREEN AND CONTROLS", "صدا، تصویر و کنترل‌ها"),
    ("MUSIC", "موسیقی"),
    ("SOUND FX", "جلوه‌های صوتی"),
    ("LANGUAGE", "زبان"),
    ("ENGLISH", "انگلیسی"),
    ("PERSIAN", "فارسی"),
    ("SCREEN SHAKE", "لرزش صفحه"),
    ("SHOW FPS", "نمایش فریم"),
    ("CONTROL BINDINGS", "تنظیم کلیدها"),
    (
        "UP/DOWN SELECT  ·  LEFT/RIGHT ADJUST  ·  ENTER CONFIRM",
        "بالا/پایین انتخاب  ·  چپ/راست تنظیم  ·  اینتر تأیید",
    ),
    (
        "CLICK A ROW, THEN PRESS A KEY",
        "یک ردیف را بزن، بعد کلید را فشار بده",
    ),
    ("LEFT", "چپ"),
    ("RIGHT", "راست"),
    ("JUMP", "پرش"),
    ("ABILITY", "توانایی"),
    ("PRESS A KEY...", "کلید را فشار بده..."),
    ("RESET DEFAULTS", "بازگردانی پیش‌فرض"),
    ("DONE", "تمام"),
    ("CONTROLS", "کنترل‌ها"),
    ("P1  ARROWS + SPACE", "P1  کلیدهای جهت + فاصله"),
    ("P2  A / D + W + LEFT CTRL", "P2  A / D + W + Ctrl چپ"),
    ("PAUSE  ESC", "توقف  ESC"),
    ("ABILITY  ONE PER SLIME", "توانایی  یکی برای هر اسلایم"),
    ("MATCH FLOW", "روند بازی"),
    (
        "FIRST TO THE TARGET SCORE WINS.",
        "اولین کسی که به امتیاز هدف برسد می‌برد.",
    ),
    (
        "AFTER EACH POINT BOTH SIDES RESET.",
        "بعد از هر امتیاز دو طرف از نو شروع می‌کنند.",
    ),
    (
        "MANA REFILLS BY MODE. BONUS",
        "مانا بر اساس حالت پر می‌شود. جایزه",
    ),
    (
        "REWARDS DEFENDING YOUR HALF.",
        "از دفاع در نیمه خودی پاداش می‌دهد.",
    ),
    ("SOCCER", "فوتبال"),
    ("VOLLEYBALL", "والیبال"),
    ("BASKETBALL", "بسکتبال"),
    (
        "ROLL THE BALL ACROSS THE GOAL LINE. THE CROSSBAR AND POSTS ARE SOLID.",
        "توپ را از خط دروازه رد کن. تیر دروازه و تیرک‌ها سخت‌اند.",
    ),
    (
        "KEEP THE BALL OFF YOUR FLOOR. THE CENTER NET BLOCKS EVERYTHING, INCLUDING DIGNITY.",
        "توپ را از زمین خودت دور نگه دار. تور وسط همه‌چیز را می‌بندد، حتی آبرو را.",
    ),
    (
        "DROP THE BALL THROUGH THE RIM FROM ABOVE. RIM SHOTS DO NOT COUNT.",
        "توپ را از بالا داخل حلقه بینداز. برخورد به حلقه حساب نیست.",
    ),
    (
        "EVERY SLIME HAS A UNIQUE ABILITY.",
        "هر اسلایم یک توانایی خاص دارد.",
    ),
    (
        "CHAMPIONSHIP: WIN THREE IN A ROW.",
        "قهرمانی: سه برد پیاپی.",
    ),
    (
        "SPACE / ENTER CONFIRM  ·  ESC BACK",
        "فاصله / اینتر تأیید  ·  ESC بازگشت",
    ),
    ("ENTER SELECT  ·  ESC MENU", "اینتر انتخاب  ·  ESC منو"),
    ("ON", "روشن"),
    ("OFF", "خاموش"),
    ("SOUND OFF", "صدا خاموش"),
    ("MUTED", "بی‌صدا"),
    // Title screen.
    ("SUPER", "سوپر"),
    ("SLIME", "اسلایم"),
    (
        "SOCCER  /  VOLLEYBALL  /  BASKETBALL",
        "فوتبال  /  والیبال  /  بسکتبال",
    ),
    (
        "OPEN SOURCE REMAKE  ·  RUST + MACROQUAD",
        "بازسازی متن‌باز  ·  Rust + Macroquad",
    ),
    // Results.
    ("CHAMPION!", "قهرمان!"),
    ("ELIMINATED", "حذف شدی"),
    ("ROUND CLEARED", "دور رد شد"),
    ("YOU WIN", "بردی"),
    ("PLAYER 1 WINS", "بازیکن 1 برد"),
    ("YOU LOSE", "باختی"),
    ("PLAYER 2 WINS", "بازیکن 2 برد"),
    ("HITS", "ضربه‌ها"),
    ("ABILITIES", "توانایی‌ها"),
    ("TOP BALL SPEED", "بیشترین سرعت توپ"),
    ("STATS", "آمار"),
    ("REMATCH", "بازی دوباره"),
    ("CHANGE SETUP", "تغییر تنظیمات"),
    ("MENU", "منو"),
    ("NEW RUN", "شروع دوباره"),
    ("TRY AGAIN", "تلاش دوباره"),
    ("NEXT ROUND", "دور بعد"),
    // Match HUD, pauses and popups.
    ("FROZEN", "یخ‌زده"),
    ("TIME FROZEN", "زمان متوقف شد"),
    ("GET READY", "آماده باش"),
    ("GO!", "برو!"),
    ("FINAL POINT", "امتیاز آخر"),
    ("NEXT RALLY SOON", "رالی بعدی به‌زودی"),
    ("PAUSED", "توقف"),
    ("ESC / START  RESUME", "ESC / شروع  ادامه"),
    ("Q  QUIT TO MENU", "Q  خروج به منو"),
    ("M  MUTE", "M  بی‌صدا"),
    ("GOAL!", "گل!"),
    ("POINT!", "امتیاز!"),
    ("BASKET!", "سبد!"),
    ("NOT ENOUGH MANA", "مانا کافی نیست"),
    ("OUT OF RANGE", "خارج از برد"),
    ("NEED GROUND", "باید روی زمین باشی"),
    ("ALREADY ACTIVE", "از قبل فعال است"),
    ("RAIN!", "باران!"),
    ("TIME FLOWS", "زمان می‌گذرد"),
    ("SLAM!", "اسلم!"),
    ("DISCO!", "دیسکو!"),
    ("SHRUNK!", "کوچک شد!"),
    ("DRAINED!", "خالی شد!"),
    // Modes and categories.
    ("BONUS", "جایزه"),
    ("OFFENSIVE", "تهاجمی"),
    ("ROUND", "دوره"),
    ("EASY", "آسان"),
    ("NORMAL", "معمولی"),
    ("HARD", "سخت"),
    ("PROJECTILE", "پرتابی"),
    ("CONTROL", "کنترلی"),
    ("BUFF", "تقویتی"),
    ("FIELD", "میدانی"),
    ("INSTANT", "آنی"),
    ("NO ABILITY", "بدون توانایی"),
    // Slime names.
    ("CLASSIC", "کلاسیک"),
    ("ARCHER", "کماندار"),
    ("BOOMER", "بومر"),
    ("HOOPS", "هوپس"),
    ("BOMB", "بمب"),
    ("BOXER", "بوکسور"),
    ("BUILDER", "سازنده"),
    ("CLOCK", "ساعت"),
    ("COSMOS", "کیهان"),
    ("COWBOY", "کابوی"),
    ("DISCO", "دیسکو"),
    ("DIVER", "غواص"),
    ("CHARM", "افسون"),
    ("BLAZE", "شعله"),
    ("FISHER", "ماهی‌گیر"),
    ("GHOST", "روح"),
    ("FROST", "یخبندان"),
    ("RAINMAKER", "باران‌ساز"),
    ("MANIAC", "دیوانه"),
    ("MONK", "راهب"),
    ("MUD", "گِل"),
    ("REAPER", "داس‌مرگ"),
    ("NATURE", "طبیعت"),
    ("RETRO", "رترو"),
    ("ROBOT", "ربات"),
    ("RUNNER", "دونده"),
    ("SCIENTIST", "دانشمند"),
    ("SPONGE", "اسفنج"),
    ("TRAFFIC", "ترافیک"),
    ("TV", "تلویزیون"),
    ("TWIN", "دوقلو"),
    ("GEYSER", "آب‌فشان"),
    ("ALIEN", "فضایی"),
    // Slime blurbs.
    (
        "The original blob. No tricks, just clean football.",
        "اسلایم اصلی. بدون ترفند، فقط فوتبال تمیز.",
    ),
    (
        "Keeps one arrow nocked and one eye on the ball.",
        "یک تیر آماده دارد و یک چشمش روی توپ است.",
    ),
    (
        "Throws a boomerang. It always comes back. Usually.",
        "بومرنگ پرتاب می‌کند. همیشه برمی‌گردد. معمولاً.",
    ),
    (
        "Joined the wrong sport and refuses to admit it.",
        "ورزش اشتباهی را انتخاب کرده و قبولش هم نمی‌کند.",
    ),
    (
        "Handles every disagreement with a controlled explosion.",
        "هر اختلافی را با یک انفجار کنترل‌شده حل می‌کند.",
    ),
    (
        "Sees a ball. Sees a punching bag. Same thing.",
        "توپ می‌بیند. کیسه بوکس می‌بیند. فرقی ندارد.",
    ),
    (
        "Believes every problem needs more wall.",
        "فکر می‌کند هر مشکلی به دیوار بیشتری نیاز دارد.",
    ),
    (
        "Always knows exactly when to waste your time.",
        "همیشه دقیقاً می‌داند کِی وقتت را هدر بدهد.",
    ),
    (
        "A tiny universe with a surprisingly good left foot.",
        "یک جهان کوچک با پای چپِ به‌طرز عجیبی خوب.",
    ),
    (
        "Fastest revolver in the western half of the pitch.",
        "سریع‌ترین کلانترِ نیمه غربی زمین.",
    ),
    (
        "Scores goals and makes sure everyone sees them.",
        "گل می‌زند و مطمئن می‌شود همه می‌بینند.",
    ),
    (
        "Brings a harpoon to a football match. Rude.",
        "به یک مسابقه فوتبال نیزه می‌آورد. بی‌ادب.",
    ),
    (
        "The ball simply likes being near her. Suspicious.",
        "توپ فقط دوست دارد نزدیکش باشد. مشکوک.",
    ),
    (
        "Runs hot. Shoots hotter. Hydration is a myth.",
        "تند می‌دود. تندتر شوت می‌زند. آب خوردن فقط یک افسانه است.",
    ),
    (
        "Patient. Methodical. Absolutely will not let go.",
        "صبور. منظم. به هیچ‌وجه ول نمی‌کند.",
    ),
    (
        "Haunts the midfield. Mostly just wants a hug.",
        "میانه میدان را تسخیر می‌کند. بیشتر از همه یک بغل می‌خواهد.",
    ),
    (
        "Plays cool. Very cool. Frosty, even.",
        "خون‌سرد بازی می‌کند. خیلی خون‌سرد. حتی یخی.",
    ),
    (
        "Brings the weather. Leaves with the ball.",
        "هوا را با خودش می‌آورد. با توپ می‌رود.",
    ),
    (
        "Tactics: none. Energy: unlimited. Fun: yes.",
        "تاکتیک: هیچ. انرژی: بی‌نهایت. تفریح: بله.",
    ),
    (
        "Inner peace. Outer chaos. Excellent ball control.",
        "آرامش درون. آشوب بیرون. کنترل عالی توپ.",
    ),
    (
        "Was a respectable slime before the first rainy season.",
        "قبل از اولین فصل بارانی، اسلایم آبرومندی بود.",
    ),
    (
        "Takes your speed. Tips the hat. Scores anyway.",
        "سرعتت را می‌گیرد. کلاه برمی‌دارد. در هر حال گل می‌زند.",
    ),
    (
        "Every match has a soundtrack. It is very loud.",
        "هر مسابقه‌ای موسیقی دارد. خیلی هم بلند.",
    ),
    (
        "Grows a tree. Waits patiently. Blocks your shot.",
        "درختی می‌کارد. صبور منتظر می‌ماند. شوتت را می‌گیرد.",
    ),
    (
        "Only has 8 bits, but uses every single one.",
        "فقط 8 بیت دارد، اما از هر بیتش استفاده می‌کند.",
    ),
    (
        "Beep. Calculates ball trajectories. Mostly.",
        "بیپ. مسیر توپ را حساب می‌کند. بیشتر وقت‌ها.",
    ),
    (
        "Has never once been late to a tackle.",
        "هیچ‌وقت برای یک تکل دیر نکرده است.",
    ),
    (
        "Peer-reviewed methods, questionable ethics.",
        "روش‌های داوری علمی، اخلاقیات مشکوک.",
    ),
    (
        "Absorbs pressure, tackles, and the occasional goal.",
        "فشار و تکل را جذب می‌کند، گاهی هم یک گل.",
    ),
    (
        "Jump first. Ask questions if you land.",
        "اول بپر. اگر فرود آمدی سؤال بپرس.",
    ),
    (
        "Stops the ball first. Asks questions never.",
        "اول توپ را نگه می‌دارد. هیچ‌وقت سؤال نمی‌پرسد.",
    ),
    (
        "Broadcasting live from the center circle.",
        "پخش زنده از دایره وسط زمین.",
    ),
    (
        "Two slimes. One brain cell. Perfect teamwork.",
        "دو اسلایم. یک سلول مغزی. کار تیمی بی‌نقص.",
    ),
    (
        "Part slime, part plumbing emergency.",
        "نیمه اسلایم، نیمه فوریت لوله‌کشی.",
    ),
    (
        "Has never heard of the offside rule. Or Earth.",
        "قانون آفساید را هیچ‌وقت نشنیده. زمین را هم نه.",
    ),
    // Ability names.
    ("BOW & ARROW", "تیر و کمان"),
    ("BOOMERANG", "بومرنگ"),
    ("DUNK", "دانک"),
    ("BOXING GLOVE", "دستکش بوکس"),
    ("BRICK WALL", "دیوار آجری"),
    ("TIME STOP", "توقف زمان"),
    ("BLACK HOLE", "سیاه‌چاله"),
    ("REVOLVER", "هفت‌تیر"),
    ("DISCO BALL", "توپ آینه‌ای"),
    ("HARPOON", "نیزه"),
    ("ATTRACTION", "جاذبه"),
    ("FIREBALL", "گلوله آتش"),
    ("FISHING HOOK", "قلاب ماهیگیری"),
    ("INVISIBILITY", "نامرئی شدن"),
    ("FROST BEAM", "پرتو یخ"),
    ("RAIN DANCE", "رقص باران"),
    ("CRAZY MODE", "حالت دیوانه"),
    ("TELEKINESIS", "تله‌کینزی"),
    ("MUD SPLASH", "پاشیدن گِل"),
    ("SOUL DRAIN", "مکیدن روح"),
    ("SOUND WAVE", "موج صدا"),
    ("TREE", "درخت"),
    ("PIXEL POWER", "قدرت پیکسلی"),
    ("MAGNET", "آهن‌ربا"),
    ("SPRINT", "سرعت"),
    ("SHRINK RAY", "پرتو کوچک‌کننده"),
    ("GROW", "بزرگ شدن"),
    ("SUPER JUMP", "پرش ابری"),
    ("STOP SIGNS", "تابلوهای ایست"),
    ("REMOTE CONTROL", "کنترل از راه دور"),
    ("CLONE", "همزاد"),
    ("TELEPORT", "دورنوردی"),
    // Ability descriptions.
    (
        "Pure fundamentals. Every drop of mana stays in the tank.",
        "مهارت خالص. هر قطره مانا در مخزن می‌ماند.",
    ),
    (
        "Fires an arrow that sends the ball flying.",
        "تیری شلیک می‌کند که توپ را به پرواز درمی‌آورد.",
    ),
    (
        "A curving throw that flies out, turns and comes back for seconds.",
        "پرتابی خمیده که می‌رود، برمی‌گردد و دوباره می‌زند.",
    ),
    (
        "Leaps at the ball and slams it toward goal.",
        "به سمت توپ می‌پرد و آن را به سوی دروازه می‌کوبد.",
    ),
    (
        "Lobs an explosive that blasts the ball away.",
        "یک ماده منفجره پرتاب می‌کند که توپ را دور می‌کند.",
    ),
    (
        "Throws a spring-loaded glove straight through the ball.",
        "دستکشی فنری را مستقیم از میان توپ رد می‌کند.",
    ),
    (
        "Builds a temporary wall. Cheap, rude, effective.",
        "یک دیوار موقت می‌سازد. ارزان، بی‌ادبانه، مؤثر.",
    ),
    (
        "Freezes the ball and the opponent for a moment. You keep moving.",
        "توپ و حریف را لحظه‌ای منجمد می‌کند. خودت حرکت می‌کنی.",
    ),
    (
        "Opens a gravity well that drags the ball off course.",
        "چاهی گرانشی باز می‌کند که توپ را از مسیر می‌کشد.",
    ),
    (
        "Fires a quick shot that really moves the ball.",
        "گلوله‌ای سریع شلیک می‌کند که حسابی توپ را جابه‌جا می‌کند.",
    ),
    (
        "Spawns a spinning mirror ball that smacks the ball around.",
        "یک توپ آینه‌ای چرخان ایجاد می‌کند که توپ را این‌طرف و آن‌طرف می‌زند.",
    ),
    (
        "Spears the ball and drags it on a taut line.",
        "توپ را نیزه می‌زند و با ریسمانی کشیده می‌کشدش.",
    ),
    (
        "Beckons the ball toward her for a short moment.",
        "برای لحظه‌ای توپ را به سمت خودش می‌کشد.",
    ),
    (
        "Launches a sizzling ball of flame at the football.",
        "یک گلوله آتشین و سوزان به سوی توپ پرتاب می‌کند.",
    ),
    (
        "Hooks the ball and reels it toward you.",
        "توپ را قلاب می‌کند و به سمت خودت می‌کشد.",
    ),
    (
        "Fades out of sight for four seconds.",
        "چهار ثانیه از دید ناپدید می‌شود.",
    ),
    (
        "A freezing burst that slows the ball and anyone nearby.",
        "انفجاری یخی که توپ و هر کسی را در نزدیکی کند می‌کند.",
    ),
    (
        "Summons rain that makes the pitch slick for everyone.",
        "باران می‌آورد و زمین را برای همه لیز می‌کند.",
    ),
    (
        "Goes absolutely feral: faster and much harder to handle.",
        "کاملاً وحشی می‌شود: سریع‌تر و خیلی سخت‌تر.",
    ),
    (
        "Reverses the ball with an unseen shove.",
        "توپ را با نیرویی نامرئی برمی‌گرداند.",
    ),
    (
        "Leaves a sticky puddle that ruins everyone's day.",
        "یک گودال چسبناک می‌گذارد که روز همه را خراب می‌کند.",
    ),
    (
        "Siphons the opponent's speed and wears it like a scarf.",
        "سرعت حریف را می‌مکد و مثل شال می‌پوشد.",
    ),
    (
        "Blasts a ring of bass that shoves the ball and opponent.",
        "حلقه‌ای از بیس می‌فرستد که توپ و حریف را هل می‌دهد.",
    ),
    (
        "Grows a huge tree in front of the ball.",
        "درختی بزرگ جلوی توپ می‌رویاند.",
    ),
    (
        "Overclocks into a chunky, over-powered sprite.",
        "به یک اسپرایت درشت و قدرتمند تبدیل می‌شود.",
    ),
    (
        "Magnetically drags the ball into kicking range.",
        "توپ را مغناطیسی به برد شوت می‌کشد.",
    ),
    (
        "Turns on the jets for four seconds.",
        "چهار ثانیه جت می‌گذارد.",
    ),
    (
        "Temporarily shrinks the opponent. For science.",
        "حریف را موقتاً کوچک می‌کند. برای علم.",
    ),
    (
        "Inflates into a wall of slime with a huge hit box.",
        "به دیواری از اسلایم با هیتباکس بزرگ تبدیل می‌شود.",
    ),
    (
        "An extra-high jump with a tiny cape flutter.",
        "پرشی بسیار بلند با تکان کوچک شنل.",
    ),
    (
        "Plants two temporary signs in the ball's path.",
        "دو تابلو موقت در مسیر توپ می‌کارد.",
    ),
    (
        "Steers the ball toward the opponent's goal by remote.",
        "توپ را از راه دور به سمت دروازه حریف هدایت می‌کند.",
    ),
    (
        "Spawns a mirror twin that repeats your moves.",
        "یک همزاد می‌سازد که حرکت‌هایت را تکرار می‌کند.",
    ),
    (
        "Erupts under the ball and launches it skyward.",
        "زیر توپ فوران می‌کند و آن را به آسمان می‌فرستد.",
    ),
    (
        "Blinks across the pitch in a puff of static.",
        "با یک جرقه ایستا از آن سوی زمین ظاهر می‌شود.",
    ),
    // Arena names and descriptions.
    ("CLASSIC PITCH", "زمین کلاسیک"),
    ("CASTLE COURT", "حیاط قلعه"),
    ("DUNE DERBY", "مسابقه تپه‌های شنی"),
    ("MOSSY GROVE", "بیشه خزه‌ای"),
    ("LAB 7", "آزمایشگاه 7"),
    ("MOON BASE", "پایگاه ماه"),
    ("RUNNER TRACK", "پیست دوی"),
    ("NEON DISCO", "دیسکوی نئون"),
    ("DEEP DIVE", "غواصی عمیق"),
    ("CITY ROOFTOP", "پشت‌بام شهر"),
    ("SNOW DAY", "روز برفی"),
    (
        "Fresh grass, bright sky, absolutely no distractions.",
        "چمن تازه، آسمان روشن، هیچ حواس‌پرتی.",
    ),
    (
        "A torch-lit stone yard with dramatic acoustics.",
        "حیاطی سنگی با مشعل و آکوستیکی نمایشی.",
    ),
    (
        "Hot sand, hotter tempers. Bring water.",
        "شن داغ، اعصاب داغ‌تر. آب بیاور.",
    ),
    (
        "A quiet woodland clearing that smells like tackles.",
        "خلنگزاری آرام که بوی تکل می‌دهد.",
    ),
    (
        "Sterile tiles, questionable experiments, excellent lighting.",
        "کاشی‌های استریل، آزمایش‌های مشکوک، نور عالی.",
    ),
    (
        "A crater-side kickabout under a very dramatic sky.",
        "فوتبالی کنار دهانه آتشفشان زیر آسمانی خیلی نمایشی.",
    ),
    (
        "All lanes lead to the goal. Probably.",
        "همه خطوط به دروازه می‌رسند. احتمالاً.",
    ),
    (
        "The lights are low, the bass is high, the ball is glowing.",
        "نورها کم، بیس زیاد، توپ درخشان.",
    ),
    (
        "Bubbles, kelp, and a suspiciously heavy football.",
        "حباب‌ها، جلبک و توپی که به‌طور مشکوکی سنگین است.",
    ),
    (
        "A gritty pitch above the traffic. Mind the puddles.",
        "زمینی خشن بالای ترافیک. مراقب گودال‌ها باش.",
    ),
    (
        "Cold feet, warm hearts, a ball that refuses to sit still.",
        "پای سرد، قلب گرم، توپی که حاضر نیست آرام بماند.",
    ),
    // Sport taglines and mana hints (used by data accessors and future screens).
    (
        "Classic slime football. Two goals, zero rules about dignity.",
        "فوتبال کلاسیک اسلایم. دو دروازه، صفر قانون برای آبرو.",
    ),
    (
        "Keep the ball off your floor. The net is not a suggestion.",
        "توپ را از زمین خودت دور نگه دار. تور فقط یک پیشنهاد نیست.",
    ),
    (
        "Hoops, headbutts and questionable jump shots.",
        "حلقه‌ها، ضربه‌های سر و شوت‌های پرشی مشکوک.",
    ),
    (
        "Base regen, three times faster in the opponent's half.",
        "بازیابی پایه، در نیمهٔ حریف سه برابر سریع‌تر.",
    ),
    (
        "Charges only while you are in the opponent's half.",
        "فقط در نیمهٔ حریف شارژ می‌شود.",
    ),
    (
        "Slow regen, faster in the opponent's half; refilled after each point.",
        "بازیابی کند، در نیمهٔ حریف سریع‌تر؛ بعد از هر امتیاز پر می‌شود.",
    ),
    // Online and LAN.
    ("ONLINE", "آنلاین"),
    ("ONLINE PLAY", "بازی آنلاین"),
    ("LAN + INTERNET", "شبکه محلی + اینترنت"),
    ("HOST GAME", "میزبانی بازی"),
    ("JOIN BY IP", "ورود با آی‌پی"),
    ("LAN GAMES", "بازی‌های شبکه محلی"),
    ("HOST ONLINE MATCH", "میزبانی بازی آنلاین"),
    ("PLAYER 2 JOINS OVER THE NETWORK", "بازیکن 2 از طریق شبکه اضافه می‌شود"),
    ("REMOTE RIVAL", "حریف راه دور"),
    ("ONLINE MATCH", "بازی آنلاین"),
    ("HOST MATCH", "میزبانی بازی"),
    ("JOIN GAME", "ورود به بازی"),
    ("TYPE THE HOST ADDRESS", "آدرس میزبان را وارد کن"),
    ("HOST ADDRESS", "آدرس میزبان"),
    ("CONNECT", "اتصال"),
    ("CONNECTING...", "در حال اتصال..."),
    ("CONTACTING THE HOST", "در حال تماس با میزبان"),
    ("YOUR SLIME", "اسلایم تو"),
    ("ARROW KEYS CHANGE SLIME", "کلیدهای جهت اسلایم را عوض می‌کنند"),
    ("ENTER CONNECT  ·  ESC BACK", "اینتر اتصال  ·  ESC بازگشت"),
    ("SEARCHING THE LOCAL NETWORK...", "جست‌وجوی شبکه محلی..."),
    ("MAKE SURE THE HOST HAS A MATCH OPEN.", "مطمئن شو میزبان یک بازی باز کرده است."),
    (
        "SOME FIREWALLS BLOCK LAN DISCOVERY. JOIN BY IP INSTEAD.",
        "برخی فایروال‌ها کشف شبکه محلی را می‌بندند. با آی‌پی وارد شو.",
    ),
    ("OPEN", "باز"),
    ("FULL", "پر"),
    ("IN MATCH", "در حال بازی"),
    ("AUTO-DISCOVERY", "کشف خودکار"),
    ("THAT MATCH IS ALREADY FULL.", "آن بازی از قبل پر است."),
    ("JOINER PICKS THEIR OWN SLIME", "بازیکن دوم اسلایم خودش را انتخاب می‌کند"),
    (
        "THE HOST RUNS THE MATCH. EVERYONE ELSE SENDS INPUT.",
        "میزبان بازی را اجرا می‌کند. بقیه ورودی می‌فرستند.",
    ),
    (
        "UDP  ·  120 HZ SIM  ·  30 HZ SNAPSHOTS  ·  PORT 43210",
        "UDP  ·  شبیه‌سازی 120 هرتز  ·  30 اسنپ‌شات  ·  پورت 43210",
    ),
    (
        "HOST A MATCH OR FIND ONE ON YOUR NETWORK.",
        "یک بازی میزبانی کن یا یکی در شبکه‌ات پیدا کن.",
    ),
    ("HOSTING ONLINE MATCH", "میزبانی بازی آنلاین"),
    ("WAITING FOR A RIVAL...", "در انتظار حریف..."),
    ("SHARE THIS ADDRESS", "این آدرس را به اشتراک بگذار"),
    ("RIVAL JOINED", "حریف پیوست"),
    ("RIVAL SLOT", "جای حریف"),
    ("OPEN SLOT", "جای خالی"),
    ("READY", "آماده"),
    (
        "LAN PLAYERS FIND YOU AUTOMATICALLY. INTERNET PLAYERS CAN TYPE YOUR ADDRESS.",
        "بازیکنان شبکه محلی خودکار تو را پیدا می‌کنند. بازیکنان اینترنت می‌توانند آدرست را وارد کنند.",
    ),
    ("ESC CANCEL", "ESC لغو"),
    ("WAITING FOR THE HOST TO START", "در انتظار شروع توسط میزبان"),
    (
        "THE MATCH STARTS AS SOON AS BOTH SLIMES ARE IN.",
        "بازی به‌محض حضور هر دو اسلایم شروع می‌شود.",
    ),
    ("SERVER", "سرور"),
    ("ONLINE LOBBY", "لابی آنلاین"),
    (
        "MAKE SURE THE HOST IS RUNNING AND UDP 43210 IS OPEN.",
        "مطمئن شو میزبان اجرا شده و پورت UDP 43210 باز است.",
    ),
    ("CTRL+C / ESC TO GIVE UP", "CTRL+C / ESC برای انصراف"),
    ("ONLINE MENU", "منوی آنلاین"),
    ("MATCH COMPLETE", "بازی تمام شد"),
    ("THE MATCH KEEPS RUNNING", "بازی همچنان ادامه دارد"),
    ("HOST", "میزبان"),
    ("GUEST", "مهمان"),
    ("BEST EFFORT UDP", "UDP با بهترین تلاش"),
    ("LEFT MATCH", "بازی رها شد"),
    ("LEFT LOBBY", "لابی رها شد"),
    ("HOSTING CANCELLED", "میزبانی لغو شد"),
];

// Isolated, final, initial and medial forms from Arabic Presentation Forms-B.
// The table is sorted by the base character and is used to avoid a text
// shaping dependency: macroquad renders TTF glyphs one by one, without GSUB.
const FORMS: &[(char, [char; 4])] = &[
    ('\u{0621}', ['\u{FE80}', '\u{0000}', '\u{0000}', '\u{0000}']),
    ('\u{0622}', ['\u{FE81}', '\u{FE82}', '\u{0000}', '\u{0000}']),
    ('\u{0623}', ['\u{FE83}', '\u{FE84}', '\u{0000}', '\u{0000}']),
    ('\u{0624}', ['\u{FE85}', '\u{FE86}', '\u{0000}', '\u{0000}']),
    ('\u{0625}', ['\u{FE87}', '\u{FE88}', '\u{0000}', '\u{0000}']),
    ('\u{0626}', ['\u{FE89}', '\u{FE8A}', '\u{FE8B}', '\u{FE8C}']),
    ('\u{0627}', ['\u{FE8D}', '\u{FE8E}', '\u{0000}', '\u{0000}']),
    ('\u{0628}', ['\u{FE8F}', '\u{FE90}', '\u{FE91}', '\u{FE92}']),
    ('\u{0629}', ['\u{FE93}', '\u{FE94}', '\u{0000}', '\u{0000}']),
    ('\u{062A}', ['\u{FE95}', '\u{FE96}', '\u{FE97}', '\u{FE98}']),
    ('\u{062B}', ['\u{FE99}', '\u{FE9A}', '\u{FE9B}', '\u{FE9C}']),
    ('\u{062C}', ['\u{FE9D}', '\u{FE9E}', '\u{FE9F}', '\u{FEA0}']),
    ('\u{062D}', ['\u{FEA1}', '\u{FEA2}', '\u{FEA3}', '\u{FEA4}']),
    ('\u{062E}', ['\u{FEA5}', '\u{FEA6}', '\u{FEA7}', '\u{FEA8}']),
    ('\u{062F}', ['\u{FEA9}', '\u{FEAA}', '\u{0000}', '\u{0000}']),
    ('\u{0630}', ['\u{FEAB}', '\u{FEAC}', '\u{0000}', '\u{0000}']),
    ('\u{0631}', ['\u{FEAD}', '\u{FEAE}', '\u{0000}', '\u{0000}']),
    ('\u{0632}', ['\u{FEAF}', '\u{FEB0}', '\u{0000}', '\u{0000}']),
    ('\u{0633}', ['\u{FEB1}', '\u{FEB2}', '\u{FEB3}', '\u{FEB4}']),
    ('\u{0634}', ['\u{FEB5}', '\u{FEB6}', '\u{FEB7}', '\u{FEB8}']),
    ('\u{0635}', ['\u{FEB9}', '\u{FEBA}', '\u{FEBB}', '\u{FEBC}']),
    ('\u{0636}', ['\u{FEBD}', '\u{FEBE}', '\u{FEBF}', '\u{FEC0}']),
    ('\u{0637}', ['\u{FEC1}', '\u{FEC2}', '\u{FEC3}', '\u{FEC4}']),
    ('\u{0638}', ['\u{FEC5}', '\u{FEC6}', '\u{FEC7}', '\u{FEC8}']),
    ('\u{0639}', ['\u{FEC9}', '\u{FECA}', '\u{FECB}', '\u{FECC}']),
    ('\u{063A}', ['\u{FECD}', '\u{FECE}', '\u{FECF}', '\u{FED0}']),
    ('\u{0641}', ['\u{FED1}', '\u{FED2}', '\u{FED3}', '\u{FED4}']),
    ('\u{0642}', ['\u{FED5}', '\u{FED6}', '\u{FED7}', '\u{FED8}']),
    ('\u{0643}', ['\u{FED9}', '\u{FEDA}', '\u{FEDB}', '\u{FEDC}']),
    ('\u{0644}', ['\u{FEDD}', '\u{FEDE}', '\u{FEDF}', '\u{FEE0}']),
    ('\u{0645}', ['\u{FEE1}', '\u{FEE2}', '\u{FEE3}', '\u{FEE4}']),
    ('\u{0646}', ['\u{FEE5}', '\u{FEE6}', '\u{FEE7}', '\u{FEE8}']),
    ('\u{0647}', ['\u{FEE9}', '\u{FEEA}', '\u{FEEB}', '\u{FEEC}']),
    ('\u{0648}', ['\u{FEED}', '\u{FEEE}', '\u{0000}', '\u{0000}']),
    ('\u{0649}', ['\u{FEEF}', '\u{FEF0}', '\u{FBE8}', '\u{FBE9}']),
    ('\u{064A}', ['\u{FEF1}', '\u{FEF2}', '\u{FEF3}', '\u{FEF4}']),
    ('\u{0671}', ['\u{FB50}', '\u{FB51}', '\u{0000}', '\u{0000}']),
    ('\u{0677}', ['\u{FBDD}', '\u{0000}', '\u{0000}', '\u{0000}']),
    ('\u{0679}', ['\u{FB66}', '\u{FB67}', '\u{FB68}', '\u{FB69}']),
    ('\u{067A}', ['\u{FB5E}', '\u{FB5F}', '\u{FB60}', '\u{FB61}']),
    ('\u{067B}', ['\u{FB52}', '\u{FB53}', '\u{FB54}', '\u{FB55}']),
    ('\u{067E}', ['\u{FB56}', '\u{FB57}', '\u{FB58}', '\u{FB59}']),
    ('\u{067F}', ['\u{FB62}', '\u{FB63}', '\u{FB64}', '\u{FB65}']),
    ('\u{0680}', ['\u{FB5A}', '\u{FB5B}', '\u{FB5C}', '\u{FB5D}']),
    ('\u{0683}', ['\u{FB76}', '\u{FB77}', '\u{FB78}', '\u{FB79}']),
    ('\u{0684}', ['\u{FB72}', '\u{FB73}', '\u{FB74}', '\u{FB75}']),
    ('\u{0686}', ['\u{FB7A}', '\u{FB7B}', '\u{FB7C}', '\u{FB7D}']),
    ('\u{0687}', ['\u{FB7E}', '\u{FB7F}', '\u{FB80}', '\u{FB81}']),
    ('\u{0688}', ['\u{FB88}', '\u{FB89}', '\u{0000}', '\u{0000}']),
    ('\u{068C}', ['\u{FB84}', '\u{FB85}', '\u{0000}', '\u{0000}']),
    ('\u{068D}', ['\u{FB82}', '\u{FB83}', '\u{0000}', '\u{0000}']),
    ('\u{068E}', ['\u{FB86}', '\u{FB87}', '\u{0000}', '\u{0000}']),
    ('\u{0691}', ['\u{FB8C}', '\u{FB8D}', '\u{0000}', '\u{0000}']),
    ('\u{0698}', ['\u{FB8A}', '\u{FB8B}', '\u{0000}', '\u{0000}']),
    ('\u{06A4}', ['\u{FB6A}', '\u{FB6B}', '\u{FB6C}', '\u{FB6D}']),
    ('\u{06A6}', ['\u{FB6E}', '\u{FB6F}', '\u{FB70}', '\u{FB71}']),
    ('\u{06A9}', ['\u{FB8E}', '\u{FB8F}', '\u{FB90}', '\u{FB91}']),
    ('\u{06AD}', ['\u{FBD3}', '\u{FBD4}', '\u{FBD5}', '\u{FBD6}']),
    ('\u{06AF}', ['\u{FB92}', '\u{FB93}', '\u{FB94}', '\u{FB95}']),
    ('\u{06B1}', ['\u{FB9A}', '\u{FB9B}', '\u{FB9C}', '\u{FB9D}']),
    ('\u{06B3}', ['\u{FB96}', '\u{FB97}', '\u{FB98}', '\u{FB99}']),
    ('\u{06BA}', ['\u{FB9E}', '\u{FB9F}', '\u{0000}', '\u{0000}']),
    ('\u{06BB}', ['\u{FBA0}', '\u{FBA1}', '\u{FBA2}', '\u{FBA3}']),
    ('\u{06BE}', ['\u{FBAA}', '\u{FBAB}', '\u{FBAC}', '\u{FBAD}']),
    ('\u{06C0}', ['\u{FBA4}', '\u{FBA5}', '\u{0000}', '\u{0000}']),
    ('\u{06C1}', ['\u{FBA6}', '\u{FBA7}', '\u{FBA8}', '\u{FBA9}']),
    ('\u{06C5}', ['\u{FBE0}', '\u{FBE1}', '\u{0000}', '\u{0000}']),
    ('\u{06C6}', ['\u{FBD9}', '\u{FBDA}', '\u{0000}', '\u{0000}']),
    ('\u{06C7}', ['\u{FBD7}', '\u{FBD8}', '\u{0000}', '\u{0000}']),
    ('\u{06C8}', ['\u{FBDB}', '\u{FBDC}', '\u{0000}', '\u{0000}']),
    ('\u{06C9}', ['\u{FBE2}', '\u{FBE3}', '\u{0000}', '\u{0000}']),
    ('\u{06CB}', ['\u{FBDE}', '\u{FBDF}', '\u{0000}', '\u{0000}']),
    ('\u{06CC}', ['\u{FBFC}', '\u{FBFD}', '\u{FBFE}', '\u{FBFF}']),
    ('\u{06D0}', ['\u{FBE4}', '\u{FBE5}', '\u{FBE6}', '\u{FBE7}']),
    ('\u{06D2}', ['\u{FBAE}', '\u{FBAF}', '\u{0000}', '\u{0000}']),
    ('\u{06D3}', ['\u{FBB0}', '\u{FBB1}', '\u{0000}', '\u{0000}']),
];

fn forms(ch: char) -> Option<&'static [char; 4]> {
    FORMS
        .binary_search_by_key(&(ch as u32), |entry| entry.0 as u32)
        .ok()
        .map(|index| &FORMS[index].1)
}

fn can_join_backward(ch: char) -> bool {
    forms(ch)
        .map(|f| f[1] != '\0' || f[2] != '\0' || f[3] != '\0')
        .unwrap_or(false)
}

fn can_join_forward(ch: char) -> bool {
    forms(ch)
        .map(|f| f[2] != '\0' || f[3] != '\0')
        .unwrap_or(false)
}

fn is_transparent(ch: char) -> bool {
    let cp = ch as u32;
    (0x0610..=0x061A).contains(&cp)
        || (0x064B..=0x065F).contains(&cp)
        || cp == 0x0670
        || (0x06D6..=0x06ED).contains(&cp)
}

fn prev_joins(chars: &[char], index: usize) -> bool {
    let current = chars[index];
    if !can_join_backward(current) {
        return false;
    }
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let candidate = chars[cursor];
        if is_transparent(candidate) {
            continue;
        }
        if candidate == '\u{200C}' {
            return false;
        }
        return can_join_forward(candidate);
    }
    false
}

fn next_joins(chars: &[char], index: usize) -> bool {
    let current = chars[index];
    if !can_join_forward(current) {
        return false;
    }
    for &candidate in chars.iter().skip(index + 1) {
        if is_transparent(candidate) {
            continue;
        }
        if candidate == '\u{200C}' {
            return false;
        }
        return can_join_backward(candidate);
    }
    false
}

fn next_significant(chars: &[char], index: usize) -> Option<usize> {
    for (offset, &candidate) in chars.iter().enumerate().skip(index + 1) {
        if !is_transparent(candidate) {
            return Some(offset);
        }
    }
    None
}

fn lam_alef(ch: char) -> Option<(char, char)> {
    match ch {
        '\u{0622}' => Some(('\u{FEF5}', '\u{FEF6}')),
        '\u{0623}' => Some(('\u{FEF7}', '\u{FEF8}')),
        '\u{0625}' => Some(('\u{FEF9}', '\u{FEFA}')),
        '\u{0627}' => Some(('\u{FEFB}', '\u{FEFC}')),
        _ => None,
    }
}

fn shape_arabic(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\u{200C}' || ch == '\u{200D}' {
            index += 1;
            continue;
        }
        if ch == '\u{0644}' {
            if let Some(next_index) = next_significant(&chars, index) {
                if next_index == index + 1 {
                    if let Some((isolated, final_form)) = lam_alef(chars[next_index]) {
                        out.push(if prev_joins(&chars, index) {
                            final_form
                        } else {
                            isolated
                        });
                        index += 2;
                        continue;
                    }
                }
            }
        }
        if let Some(forms) = forms(ch) {
            let connected_previous = prev_joins(&chars, index);
            let connected_next = next_joins(&chars, index);
            let shape = match (connected_previous, connected_next) {
                (true, true) => forms[3],
                (true, false) => forms[1],
                (false, true) => forms[2],
                // The font omits isolated presentation forms; the base letter
                // has exactly that shape.
                (false, false) => '\0',
            };
            out.push(if shape == '\0' { ch } else { shape });
        } else {
            out.push(ch);
        }
        index += 1;
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Rtl,
    Ltr,
    Neutral,
}

fn direction(ch: char) -> Direction {
    let cp = ch as u32;
    if (0x0660..=0x0669).contains(&cp) || (0x06F0..=0x06F9).contains(&cp) {
        return Direction::Ltr;
    }
    if (0x0600..=0x06FF).contains(&cp)
        || (0x0750..=0x077F).contains(&cp)
        || (0x08A0..=0x08FF).contains(&cp)
        || (0xFB50..=0xFDFF).contains(&cp)
        || (0xFE70..=0xFEFF).contains(&cp)
    {
        if is_transparent(ch) || matches!(cp, 0x060C | 0x061B | 0x061F | 0x066A..=0x066D | 0x06D4) {
            return Direction::Neutral;
        }
        return Direction::Rtl;
    }
    if ch.is_ascii_alphanumeric() || (0x00C0..=0x024F).contains(&cp) {
        return Direction::Ltr;
    }
    if matches!(ch, '%' | '+') {
        return Direction::Ltr;
    }
    Direction::Neutral
}

fn mirror(ch: char) -> char {
    match ch {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '«' => '»',
        '»' => '«',
        other => other,
    }
}

fn reorder_bidi(shaped: &str) -> String {
    let chars: Vec<char> = shaped
        .chars()
        .filter(|ch| *ch != '\u{200C}' && *ch != '\u{200D}')
        .collect();
    if !chars.iter().any(|ch| direction(*ch) == Direction::Rtl) {
        return chars.into_iter().collect();
    }

    let mut directions: Vec<Direction> = chars.iter().map(|ch| direction(*ch)).collect();
    let mut index = 0;
    while index < directions.len() {
        if directions[index] != Direction::Neutral {
            index += 1;
            continue;
        }
        let start = index;
        while index < directions.len() && directions[index] == Direction::Neutral {
            index += 1;
        }
        let before = if start == 0 {
            Direction::Rtl
        } else {
            directions[start - 1]
        };
        let after = if index == directions.len() {
            Direction::Rtl
        } else {
            directions[index]
        };
        let resolved = if before == after {
            before
        } else {
            Direction::Rtl
        };
        for slot in &mut directions[start..index] {
            *slot = resolved;
        }
    }

    let mut runs: Vec<(Direction, String)> = Vec::new();
    for (ch, dir) in chars.iter().zip(directions.iter()) {
        match runs.last_mut() {
            Some((last_dir, run)) if last_dir == dir => run.push(*ch),
            _ => runs.push((*dir, ch.to_string())),
        }
    }

    let mut out = String::with_capacity(shaped.len());
    for (dir, run) in runs.iter().rev() {
        if *dir == Direction::Rtl {
            for ch in run.chars().rev() {
                out.push(mirror(ch));
            }
        } else {
            out.push_str(run);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_table_has_unique_keys_and_persian_values() {
        let mut seen: HashMap<String, &str> = HashMap::new();
        for (english, persian) in PHRASES {
            let key = english.to_ascii_lowercase();
            if let Some(previous) = seen.insert(key.clone(), persian) {
                assert_eq!(previous, *persian, "conflicting duplicate key: {english}");
            }
            assert!(
                persian.chars().any(is_persian_letter),
                "translation has no Persian letters: {english} -> {persian}"
            );
        }
    }

    #[test]
    fn english_is_untouched() {
        assert_eq!(
            localize_for(Language::English, "QUICK MATCH"),
            "QUICK MATCH"
        );
        assert_eq!(display_line_for(Language::English, "Classic"), "Classic");
    }

    #[test]
    fn static_phrases_are_case_insensitive() {
        assert_eq!(localize_for(Language::Persian, "QUICK MATCH"), "بازی سریع");
        assert_eq!(
            localize_for(Language::Persian, "Classic Pitch"),
            "زمین کلاسیک"
        );
        assert_eq!(
            localize_for(Language::Persian, "NOT ENOUGH MANA"),
            "مانا کافی نیست"
        );
        assert_eq!(localize_for(Language::Persian, "RAIN!"), "باران!");
    }

    #[test]
    fn dynamic_patterns_translate_their_parts() {
        assert_eq!(
            localize_for(Language::Persian, "CUPS WON: 4"),
            "جام‌های فتح‌شده: 4"
        );
        assert_eq!(localize_for(Language::Persian, "P2  LEFT"), "بازیکن 2  چپ");
        assert_eq!(
            localize_for(Language::Persian, "P1  CLASSIC"),
            "بازیکن 1  کلاسیک"
        );
        assert_eq!(localize_for(Language::Persian, "6 GOALS"), "6 امتیاز");
        assert_eq!(
            localize_for(Language::Persian, "BOW & ARROW NEEDS 35"),
            "تیر و کمان نیاز به 35 دارد"
        );
    }

    #[test]
    fn framing_letters_join_into_presentation_forms() {
        assert_eq!(shape_arabic("سلام"), "\u{FEB3}\u{FEFC}\u{0645}");
        assert_eq!(shape_arabic("بب"), "\u{FE91}\u{FE90}");
        assert_eq!(shape_arabic("لا"), "\u{FEFB}");
        assert_eq!(shape_arabic("بلا"), "\u{FE91}\u{FEFC}");
    }

    #[test]
    fn persian_is_reordered_for_a_left_to_right_renderer() {
        assert_eq!(
            display_line_for(Language::Persian, "اب"),
            "\u{0628}\u{0627}"
        );
        assert_eq!(
            display_line_for(Language::Persian, "اب 1"),
            "1 \u{0628}\u{0627}"
        );
        assert_eq!(display_line_for(Language::Persian, "50%"), "50%");
        assert_eq!(display_line_for(Language::Persian, "P1"), "P1");
    }

    #[test]
    fn game_content_is_translated() {
        use crate::game::arenas::ARENAS;
        use crate::game::characters::CHARACTERS;
        use crate::game::config::{Difficulty, ManaMode, Sport};

        let mut strings: Vec<&str> = Vec::new();
        for character in &CHARACTERS {
            strings.push(character.name);
            strings.push(character.blurb);
            strings.push(character.ability.name);
            strings.push(character.ability.desc);
            strings.push(character.ability.category.label());
        }
        for arena in &ARENAS {
            strings.push(arena.name);
            strings.push(arena.desc);
        }
        for sport in Sport::ALL {
            strings.push(sport.label());
            strings.push(sport.tagline());
            strings.push(sport.point_word());
        }
        for mode in [ManaMode::Bonus, ManaMode::Offensive, ManaMode::Round] {
            strings.push(mode.label());
            strings.push(mode.hint());
        }
        for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
            strings.push(difficulty.label());
        }
        for english in strings {
            let persian = localize_for(Language::Persian, english);
            assert_ne!(persian, english, "missing translation: {english}");
            assert!(
                persian.chars().any(is_persian_letter),
                "translation is not Persian: {english} -> {persian}"
            );
        }
    }

    fn is_persian_letter(ch: char) -> bool {
        matches!(ch as u32, 0x0620..=0x06FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF) && ch != '\u{200C}'
    }
}
