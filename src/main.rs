use anyhow::{
    anyhow,
    Context as _,
};
use once_cell::sync::Lazy;
use std::{
    collections::{
        HashMap,
        HashSet,
    },
    convert::{
        TryFrom,
        TryInto,
    },
    fmt::Display,
    fs::File,
    io::Read as _,
    path::{
        Path,
        PathBuf,
    },
};
use structopt::StructOpt;

fn parse_ptr(raw: &[u8]) -> anyhow::Result<usize> {
    if raw.len() < 4 {
        return Err(anyhow!("truncated pointer"));
    }
    let (ptr_raw, _) = raw.split_at(std::mem::size_of::<u32>());
    let ptr = u32::from_le_bytes(ptr_raw.try_into().unwrap());
    Ok(ptr as usize)
}

fn parse_list(raw: &[u8]) -> anyhow::Result<&[u8]> {
    let mut i = 0;
    loop {
        if i >= raw.len() {
            return Err(anyhow!("truncated list"));
        }
        if raw[i] == 0xFF {
            break;
        }
        i += 1;
    }
    Ok(&raw[..i])
}

static CHARACTER_MAP: Lazy<HashMap<u16, &'static str>> = Lazy::new(|| {
    maplit::hashmap! {
        0x0 => "0",
        0x1 => "1",
        0x2 => "2",
        0x3 => "3",
        0x4 => "4",
        0x5 => "5",
        0x6 => "6",
        0x7 => "7",
        0x8 => "8",
        0x9 => "9",
        0x0A => "A",
        0x0B => "B",
        0x0C => "C",
        0x0D => "D",
        0x0E => "E",
        0x0F => "F",
        0x10 => "G",
        0x11 => "H",
        0x12 => "I",
        0x13 => "J",
        0x14 => "K",
        0x15 => "L",
        0x16 => "M",
        0x17 => "N",
        0x18 => "O",
        0x19 => "P",
        0x1A => "Q",
        0x1B => "R",
        0x1C => "S",
        0x1D => "T",
        0x1E => "U",
        0x1F => "V",
        0x20 => "W",
        0x21 => "X",
        0x22 => "Y",
        0x23 => "Z",
        0x24 => "a",
        0x25 => "b",
        0x26 => "c",
        0x27 => "d",
        0x28 => "e",
        0x29 => "f",
        0x2A => "g",
        0x2B => "h",
        0x2C => "i",
        0x2D => "j",
        0x2E => "k",
        0x2F => "l",
        0x30 => "m",
        0x31 => "n",
        0x32 => "o",
        0x33 => "p",
        0x34 => "q",
        0x35 => "r",
        0x36 => "s",
        0x37 => "t",
        0x38 => "u",
        0x39 => "v",
        0x3A => "w",
        0x3B => "x",
        0x3C => "y",
        0x3D => "z",
        0x41 => "<SQUARE>",
        0x44 => "?",
        0x45 => "!",
        0x46 => "/",
        0x49 => "-",
        0x54 => ",",
        0x55 => ".",
        0x56 => "",
        0x5B => "PLUS SIGN",
        0xFB => "<X>",
        0xFC => "<NEW BOX>",
        0xFD => " ",
        0xFE => "<ENTER>",
        0xF000 => "Akira",
        0xF006 => "Digimon",
        0xF007 => "you",
        0xF008 => "the",
        0xF009 => "Digi-Beetle",
        0xF00A => "Domain",
        0xF00B => "Guard",
        0xF00C => "Tamer",
        0xF00D => "here",
        0xF00E => "have",
        0xF00F => "Knights",
        0xF010 => "and",
        0xF011 => "thing",
        0xF012 => "Security",
        0xF013 => "that",
        0xF014 => "Bertran",
        0xF015 => "Tournament",
        0xF016 => "Crimson",
        0xF018 => "something",
        0xF019 => "Item",
        0xF01A => "Falcon",
        0xF01B => "for",
        0xF01C => "That's",
        0xF01D => "Commander",
        0xF01E => "Blood",
        0xF01F => "Leader",
        0xF020 => "Attendant",
        0xF021 => "Cecilia",
        0xF022 => "all",
        0xF023 => "mission",
        0xF024 => "this",
        0xF026 => "Archive",
        0xF027 => "Black",
        0xF028 => "I'll",
        0xF029 => "are",
        0xF02A => "Sword",
        0xF02B => "right",
        0xF02C => "Digivolve",
        0xF02D => "enter",
        0xF02E => "What",
        0xF02F => "will",
        0xF030 => "come",
        0xF031 => "You",
        0xF032 => "Coliseum",
        0xF033 => "about",
        0xF034 => "don't",
        0xF035 => "anything",
        0xF037 => "Parts",
        0xF038 => "where",
        0xF039 => "The",
        0xF03A => "know",
        0xF03B => "Leomon",
        0xF03C => "want",
        0xF03D => "Oldman",
        0xF03E => "like",
        0xF03F => "need",
        0xF040 => "Chief",
        0xF041 => "with",
        0xF042 => "Thank",
        0xF044 => "Island",
        0xF045 => "can",
        0xF046 => "really",
        0xF047 => "Blue",
        0xF048 => "time",
    }
});

fn parse_string_piece(
    mut raw: &[u8]
) -> anyhow::Result<Option<(&'static str, &[u8])>> {
    if raw.is_empty() {
        return Err(anyhow!("truncated character"));
    }
    let first = raw[0];
    raw = &raw[1..];
    let encoding = match first {
        0xFF => return Ok(None),
        0xF0 => {
            if raw.is_empty() {
                return Err(anyhow!("truncated character"));
            }
            let second = raw[0];
            raw = &raw[1..];
            (first as u16) << 8 | (second as u16)
        },
        first => first as u16,
    };
    Ok(Some((
        CHARACTER_MAP
            .get(&encoding)
            .copied()
            .ok_or_else(|| anyhow!("illegal character 0x{:02X}", encoding))?,
        raw,
    )))
}

fn parse_string(mut raw: &[u8]) -> anyhow::Result<String> {
    let mut value = String::new();
    while let Some((piece, rest)) = parse_string_piece(raw)? {
        value.push_str(piece);
        raw = rest;
    }
    Ok(value)
}

struct FloorPlan {
    // 48 rows, 32 columns
    tiles: [[u8; 32]; 48],
}

impl FloorPlan {
    fn new(mut raw: &[u8]) -> anyhow::Result<Self> {
        let mut tiles = [[0_u8; 32]; 48];
        tiles.iter_mut().try_for_each::<_, anyhow::Result<()>>(|row| {
            row.iter_mut().try_for_each::<_, anyhow::Result<()>>(|tile| {
                *tile = raw
                    .first()
                    .copied()
                    .ok_or_else(|| anyhow!("truncated floor plan"))?;
                raw = &raw[1..];
                Ok(())
            })
        })?;
        Ok(Self {
            tiles,
        })
    }
}

impl Display for FloorPlan {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for y in 0..48 {
            for x in 0..32 {
                if x != 0 {
                    write!(f, " ")?;
                }
                write!(f, "{:02X}", self.tiles[y][x])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

struct Layout {}

impl Layout {
    fn new(
        raw: &[u8],
        table_ptr: usize,
    ) -> anyhow::Result<Self> {
        if raw.len() < table_ptr + 20 {
            return Err(anyhow!("truncated layout pointer table"));
        }
        let floor_plan_ptr = parse_ptr(&raw[table_ptr..])
            .context("parsing floor plan pointer")?;
        let floor_plan = FloorPlan::new(&raw[floor_plan_ptr..])?;
        println!("Floor plan is at {:X}", floor_plan_ptr);
        println!("Floor plan:");
        println!("{}", floor_plan);
        let warps_ptr = parse_ptr(&raw[table_ptr + 4..])
            .context("parsing warps pointer")?;
        let chests_ptr = parse_ptr(&raw[table_ptr + 8..])
            .context("parsing chests pointer")?;
        let traps_ptr = parse_ptr(&raw[table_ptr + 12..])
            .context("parsing traps pointer")?;
        let digimon_ptr = parse_ptr(&raw[table_ptr + 16..])
            .context("parsing digimon pointer")?;
        Ok(Self {})
    }
}

struct Floor {}

impl Floor {
    fn new(
        raw: &[u8],
        table_ptr: usize,
    ) -> anyhow::Result<Self> {
        let name_ptr =
            parse_ptr(&raw[table_ptr..]).context("parsing name pointer")?;
        let title = parse_string(&raw[name_ptr..]).context("parsing name")?;
        println!("Floor: \"{}\"", title);
        if raw.len() < table_ptr + 8 {
            return Err(anyhow!(
                "floor table truncated before layout pointers"
            ));
        }
        let mut next_layout_ptr_offset = &raw[table_ptr + 8..];
        let mut layout_ptrs = HashSet::new();
        for i in 0..8 {
            let layout_ptr = parse_ptr(next_layout_ptr_offset)
                .context("parsing layout pointer")?;
            next_layout_ptr_offset = &next_layout_ptr_offset[4..];
            if layout_ptrs.insert(layout_ptr) {
                let layout = Layout::new(raw, layout_ptr)
                    .context(format!("parsing layout {}", i + 1))?;
            }
        }
        Ok(Self {})
    }
}

struct Dungeon {}

impl Dungeon {}

impl TryFrom<&[u8]> for Dungeon {
    type Error = anyhow::Error;

    fn try_from(raw: &[u8]) -> Result<Self, Self::Error> {
        println!("Dungeon raw file is {} bytes", raw.len());
        let mut floors = Vec::new();
        let mut raw_ptrs = raw;
        let mut i = 1;
        loop {
            let floor_ptr =
                parse_ptr(raw_ptrs).context("parsing next floor pointer")?;
            raw_ptrs = &raw_ptrs[4..];
            if floor_ptr == 0 {
                break;
            }
            floors.push(
                Floor::new(raw, floor_ptr)
                    .context(format!("parsing floor {}", i))?,
            );
            i += 1;
        }
        Ok(Self {})
    }
}

impl TryFrom<&Vec<u8>> for Dungeon {
    type Error = anyhow::Error;

    fn try_from(raw: &Vec<u8>) -> Result<Self, Self::Error> {
        let raw: &[u8] = &raw;
        Self::try_from(raw)
    }
}

impl TryFrom<&Path> for Dungeon {
    type Error = anyhow::Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let mut file = File::open(path)
            .context(format!("opening dungeon file \"{:?}\"", path))?;
        let mut dungeon_raw = Vec::new();
        file.read_to_end(&mut dungeon_raw).context("reading dungeon file")?;
        Self::try_from(&dungeon_raw)
    }
}

impl TryFrom<&PathBuf> for Dungeon {
    type Error = anyhow::Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let path: &Path = &path;
        Self::try_from(path)
    }
}

#[derive(Clone, StructOpt)]
struct Opts {
    /// Path to dungeon file to parse
    #[structopt(default_value = "../../data/DUNG4000.BIN")]
    dungeon_file_relative_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::from_args();
    let dungeon_file_path = std::env::current_exe()
        .context("getting program directory path")?
        .parent()
        .expect("unable to get directory containing program")
        .join(opts.dungeon_file_relative_path);
    let dungeon = Dungeon::try_from(&dungeon_file_path)
        .context("parsing dungeon file")?;
    Ok(())
}
