//! Canned demo data generation (03-features/demo-mode §5.2). Dish pools transcribed verbatim
//! from v1 demoData.ts; menu generation uses the deterministic LCG (§5.1) so results are
//! stable within a calendar day and rotate daily.
use crate::datetime::Clock;
use crate::demo::prng::Lcg;
use crate::domain::{MenuCategory, MenuItem};
use chrono::{Datelike, Duration, Local, TimeZone, Weekday};

/// A dish: (title, subtitle, allergen letters).
type Dish = (&'static str, &'static str, &'static [&'static str]);

struct Pool {
    category: MenuCategory,
    id_prefix: &'static str,
    price: &'static str,
    dishes: [Dish; 10],
}

const MENU1: Pool = Pool {
    category: MenuCategory::Menu1,
    id_prefix: "demo-m1",
    price: "6,00 €",
    dishes: [
        (
            "Wiener Schnitzel",
            "mit Petersilerdäpfel und Preiselbeeren",
            &["A", "C", "G"],
        ),
        (
            "Schweinsbraten",
            "mit Semmelknödel und Sauerkraut",
            &["A", "C", "G"],
        ),
        (
            "Tafelspitz",
            "mit Apfelkren und Schnittlauchsauce",
            &["A", "G", "L"],
        ),
        (
            "Rindsgulasch",
            "mit Nockerl und Essiggurkerl",
            &["A", "C", "G"],
        ),
        ("Backhendl", "mit Erdäpfelsalat", &["A", "C", "G"]),
        (
            "Cordon Bleu",
            "mit Reis und Preiselbeeren",
            &["A", "C", "G"],
        ),
        (
            "Zwiebelrostbraten",
            "mit Bratkartoffeln und Röstzwiebeln",
            &["A", "G", "L"],
        ),
        (
            "Faschierter Braten",
            "mit Erdäpfelpüree und Bratensauce",
            &["A", "C", "G"],
        ),
        ("Kalbsrahmgulasch", "mit Butternockerl", &["A", "C", "G"]),
        (
            "Gebackene Leber",
            "mit Erdäpfelsalat und Preiselbeeren",
            &["A", "C", "G"],
        ),
    ],
};

const MENU2: Pool = Pool {
    category: MenuCategory::Menu2,
    id_prefix: "demo-m2",
    price: "6,00 €",
    dishes: [
        ("Gemüselasagne", "mit Blattsalat", &["A", "C", "G"]),
        (
            "Spinatknödel",
            "mit Parmesan und brauner Butter",
            &["A", "C", "G"],
        ),
        (
            "Käsespätzle",
            "mit Röstzwiebeln und grünem Salat",
            &["A", "C", "G"],
        ),
        (
            "Pasta Primavera",
            "mit Saisongemüse und Basilikum",
            &["A", "C"],
        ),
        ("Kartoffelgratin", "mit buntem Gemüse", &["A", "G"]),
        (
            "Topfenknödel",
            "mit Butterbröseln und Apfelmus",
            &["A", "C", "G"],
        ),
        ("Gemüse-Curry", "mit Basmatireis und Naan-Brot", &["A", "G"]),
        (
            "Flammkuchen",
            "mit Sauerrahm, Zwiebeln und Speck",
            &["A", "G"],
        ),
        (
            "Eierschwammerlgulasch",
            "mit Semmelknödel",
            &["A", "C", "G"],
        ),
        (
            "Palatschinken",
            "mit Topfenfülle und Vanillesauce",
            &["A", "C", "G"],
        ),
    ],
};

const MENU3: Pool = Pool {
    category: MenuCategory::Menu3,
    id_prefix: "demo-m3",
    price: "6,00 €",
    dishes: [
        (
            "Grillhendl",
            "mit Pommes frites und Cole Slaw",
            &["A", "G", "M"],
        ),
        (
            "Fischfilet",
            "mit Dillsauce und Petersilerdäpfel",
            &["A", "C", "D", "G"],
        ),
        (
            "Putengeschnetzeltes",
            "mit Reis und Champignons",
            &["A", "G"],
        ),
        ("Cevapcici", "mit Djuvec-Reis und Ajvar", &["A", "C"]),
        (
            "Hühnercurry",
            "mit Jasminreis und Mango-Chutney",
            &["A", "G"],
        ),
        (
            "Leberkäse",
            "mit Spiegelei und Erdäpfelsalat",
            &["A", "C", "G"],
        ),
        ("Bratwürstel", "mit Senf und Sauerkraut", &["A", "M"]),
        (
            "Puten-Wrap",
            "mit Salat, Tomaten und Joghurt-Dressing",
            &["A", "G"],
        ),
        (
            "Lachs gegrillt",
            "mit Zitronenbutter und Gemüsereis",
            &["D", "G"],
        ),
        ("Spaghetti Bolognese", "mit Parmesan", &["A", "C", "G"]),
    ],
};

const SOUP_SALAD: Pool = Pool {
    category: MenuCategory::SoupAndSalad,
    id_prefix: "demo-ss",
    price: "2,50 €",
    dishes: [
        (
            "Frittatensuppe",
            "Klare Rindsuppe mit Frittaten",
            &["A", "C", "G"],
        ),
        (
            "Kürbiscremesuppe",
            "mit Kürbiskernöl und Croutons",
            &["A", "G"],
        ),
        ("Gemischter Salat", "mit Hausdressing", &["M"]),
        (
            "Grießnockerlsuppe",
            "Klare Suppe mit Grießnockerl",
            &["A", "C", "G"],
        ),
        (
            "Tomatencremesuppe",
            "mit Basilikum und Croutons",
            &["A", "G"],
        ),
        ("Kartoffelsuppe", "mit Einlage und Brot", &["A", "G", "L"]),
        (
            "Caesar Salad",
            "mit Hühnerstreifen und Parmesan",
            &["A", "C", "G"],
        ),
        (
            "Leberknödelsuppe",
            "Klare Rindsuppe mit Leberknödel",
            &["A", "C", "G"],
        ),
        ("Gemüsesuppe", "mit frischem Saisongemüse", &["A", "L"]),
        ("Blattsalat", "mit Kernöl-Dressing und Kürbiskernen", &["H"]),
    ],
};

/// §5.2 — 40 items (10 weekdays × 4 pools). Deterministic per calendar day via the §5.1 LCG.
pub fn generate_demo_menus(clock: &dyn Clock) -> Vec<MenuItem> {
    let now = clock.now_epoch_ms();
    let today = Local
        .timestamp_millis_opt(now)
        .single()
        .expect("valid epoch")
        .date_naive();

    // seed = year*10000 + (0-based month + 1)*100 + day  (chrono month() is already 1-based).
    let seed = today.year() as i64 * 10000 + today.month() as i64 * 100 + today.day() as i64;

    // Monday of the current week (Sunday belongs to the week that started 6 days earlier).
    let dow = today.weekday().num_days_from_sunday() as i64; // 0=Sun..6=Sat
    let diff = if dow == 0 { -6 } else { 1 - dow };
    let monday = today + Duration::days(diff);

    // the next 10 weekdays (Mon–Fri, skipping Sat/Sun) from that Monday.
    let mut days = Vec::with_capacity(10);
    let mut d = monday;
    while days.len() < 10 {
        if !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
            days.push(d);
        }
        d += Duration::days(1);
    }

    let pools = [&MENU1, &MENU2, &MENU3, &SOUP_SALAD];
    let mut rng = Lcg::new(seed);
    let mut items = Vec::with_capacity(40);
    for (day_index, date) in days.iter().enumerate() {
        // one draw per pool, IN ORDER (Menu1, Menu2, Menu3, Soup & Salad).
        for pool in pools {
            let r = rng.next_f64();
            let dish_index = (day_index + (r * 3.0).floor() as usize) % 10;
            let (title, subtitle, allergens) = pool.dishes[dish_index];
            items.push(MenuItem {
                id: format!("{}-{}", pool.id_prefix, day_index),
                day: format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()),
                title: pool.category.display().to_string(),
                subtitle: format!("{title} {subtitle}"),
                allergens: allergens.iter().map(|s| s.to_string()).collect(),
                available: true,
                ordered: false,
                category: pool.category,
                price: pool.price.to_string(),
            });
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::FixedClock;

    fn clock_on(y: i32, mo: u32, d: u32) -> FixedClock {
        let ms = Local
            .with_ymd_and_hms(y, mo, d, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_millis();
        FixedClock { epoch_ms: ms }
    }

    #[test]
    fn produces_40_items_all_available_no_weekends() {
        let items = generate_demo_menus(&clock_on(2026, 2, 10)); // a Tuesday
        assert_eq!(items.len(), 40);
        assert!(items.iter().all(|i| i.available && !i.ordered));
        // all four categories present.
        for c in [
            MenuCategory::Menu1,
            MenuCategory::Menu2,
            MenuCategory::Menu3,
            MenuCategory::SoupAndSalad,
        ] {
            assert!(items.iter().any(|i| i.category == c));
        }
        // no Saturday/Sunday days.
        for i in &items {
            let d = chrono::NaiveDate::parse_from_str(&i.day, "%Y-%m-%d").unwrap();
            assert!(!matches!(d.weekday(), Weekday::Sat | Weekday::Sun));
        }
        // id shape + title carries the category display string.
        assert!(items.iter().any(|i| i.id == "demo-m1-0"));
        assert!(items.iter().any(|i| i.title == "MENÜ I"));
        assert!(items.iter().any(|i| i.price == "2,50 €")); // soup & salad price
    }

    #[test]
    fn deterministic_within_a_day() {
        let a = generate_demo_menus(&clock_on(2026, 2, 10));
        let b = generate_demo_menus(&clock_on(2026, 2, 10));
        assert_eq!(a, b);
    }

    #[test]
    fn ten_distinct_weekdays_starting_monday() {
        let items = generate_demo_menus(&clock_on(2026, 2, 11)); // a Wednesday
        let mut days: Vec<String> = items.iter().map(|i| i.day.clone()).collect();
        days.sort();
        days.dedup();
        assert_eq!(days.len(), 10);
        // the earliest day is the Monday of that week (2026-02-09).
        assert_eq!(days[0], "2026-02-09");
    }
}
