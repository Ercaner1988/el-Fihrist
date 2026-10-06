//! Turso ile tek bir deyim yazar: G-1 kurulum betiğinin "Turso'dan yazış"
//! adımı için (`deneme/kurulum.ts`). Olmayan dosyayı yaratmaz (`ac`).
//!
//! Kullanım: `yaz <db> <sql>`; etkilenen satır sayısını basar.

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let mut a = std::env::args().skip(1);
    let (Some(db), Some(sql)) = (a.next(), a.next()) else {
        eprintln!("kullanım: yaz <db> <sql>");
        return std::process::ExitCode::from(2);
    };
    let sonuc = async {
        let c = fihrist_canli::ac(std::path::Path::new(&db)).await?;
        Ok::<u64, fihrist_canli::Hata>(c.execute(&sql, ()).await?)
    }
    .await;
    match sonuc {
        Ok(n) => {
            println!("{n}");
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("yaz: {db}: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
