use mem_db::Database;

fn main() {
    let db = Database::new();
    db.insert("user:1", "alice");
    db.insert("user:2", "bob");

    println!("Keys: {:?}", db.keys());
    println!("user:1 => {:?}", db.get("user:1"));
    println!(
        "Stats: inserts={}, deletes={}, lookups={}",
        db.stats().inserts,
        db.stats().deletes,
        db.stats().lookups
    );
}
