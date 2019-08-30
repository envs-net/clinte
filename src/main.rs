use clap;
use log::info;
use std::io;
use std::time;
use users;

mod db;
mod logging;
mod posts;

fn main() {
    let arg_matches = clap::App::new("clinte")
        .version(clap::crate_version!())
        .author("Ben Morrison (gbmor)")
        .about("Command-line community notices system")
        .subcommand(clap::SubCommand::with_name("post").about("Post a new notice"))
        .subcommand(
            clap::SubCommand::with_name("update")
                .about("Update a notice you've posted")
                .arg(clap::Arg::with_name("id").help("Numeric ID of the post")),
        )
        .subcommand(
            clap::SubCommand::with_name("delete")
                .about("Delete a notice you've posted")
                .arg(clap::Arg::with_name("id").help("Numeric ID of the post")),
        )
        .get_matches();

    let start = time::Instant::now();
    logging::init();
    info!("clinte starting up!");
    println!("clinte v{}", clap::crate_version!());
    println!("a community notices system");
    println!();

    let db = db::Conn::new();

    info!("Startup completed in {:?}ms", start.elapsed().as_millis());

    if arg_matches.subcommand_matches("post").is_some() {
        info!("New post...");
        post(&db);
    } else if arg_matches.subcommand_matches("update").is_some() {
        info!("Updating post ...");
        update(&db);
    } else if arg_matches.subcommand_matches("delete").is_some() {
        info!("Deleting post");
        delete(&db);
    }

    list_matches(&db);
}

// Make sure nobody encodes narsty characters
// into a message to negatively affect other
// users
fn str_to_utf8(str: &str) -> String {
    str.chars()
        .map(|c| {
            let mut buf = [0; 4];
            c.encode_utf8(&mut buf).to_string()
        })
        .collect::<String>()
}

fn list_matches(db: &db::Conn) {
    let mut stmt = db.conn.prepare("SELECT * FROM posts").unwrap();
    let out = stmt
        .query_map(rusqlite::NO_PARAMS, |row| {
            let id: u32 = row.get(0)?;
            let title: String = row.get(1)?;
            let author: String = row.get(2)?;
            let body: String = row.get(3)?;
            Ok(db::Post {
                id,
                title,
                author,
                body,
            })
        })
        .unwrap();

    let mut postvec = Vec::new();
    out.for_each(|row| {
        if let Ok(post) = row {
            postvec.push(format!(
                "{}. {} -> by {}\n{}\n\n",
                post.id, post.title, post.author, post.body
            ));
        }
    });

    for (i, e) in postvec.iter().enumerate() {
        if (postvec.len() > 14 && i >= postvec.len() - 15) || postvec.len() < 15 {
            print!("{}", e);
        }
    }
}

fn post(db: &db::Conn) {
    let mut stmt = db
        .conn
        .prepare("INSERT INTO posts (title, author, body) VALUES (:title, :author, :body)")
        .unwrap();

    println!();
    println!("Title of the new post: ");
    let mut title = String::new();
    io::stdin().read_line(&mut title).unwrap();
    let title = str_to_utf8(title.trim());
    let title = if title.len() > 30 {
        &title[..30]
    } else {
        &title
    };

    println!();
    println!("Body of the new post: ");
    let mut body = String::new();
    io::stdin().read_line(&mut body).unwrap();
    let body = str_to_utf8(body.trim());
    let body = if body.len() > 500 {
        &body[..500]
    } else {
        &body
    };

    posts::new(&mut stmt, title, body).unwrap();

    println!();
}

fn update(db: &db::Conn) {
    let cur_user = users::get_current_username()
        .unwrap()
        .into_string()
        .unwrap();

    println!();
    println!("ID number of your post to edit?");
    let mut id_num_in = String::new();
    io::stdin().read_line(&mut id_num_in).unwrap();
    let id_num_in: u32 = id_num_in.trim().parse().unwrap();

    let mut get_stmt = db
        .conn
        .prepare("SELECT * FROM posts WHERE id = :id")
        .unwrap();

    let row = get_stmt
        .query_row_named(&[(":id", &id_num_in)], |row| {
            let title: String = row.get(1).unwrap();
            let author = row.get(2).unwrap();
            let body = row.get(3).unwrap();
            Ok(vec![title, author, body])
        })
        .unwrap();

    if cur_user != row[1] {
        println!();
        println!("Username mismatch - can't update post!");
        return;
    }

    let mut new_title = String::new();
    let mut new_body = String::new();

    println!("Updating post {}", id_num_in);
    println!();
    println!("Title: {}\n\nBody: {}", row[0], row[2]);
    println!();
    println!("Enter new title:");
    io::stdin().read_line(&mut new_title).unwrap();
    println!();
    println!("Enter new body:");
    io::stdin().read_line(&mut new_body).unwrap();
    println!();

    posts::update(&new_title, &new_body, id_num_in, &db).unwrap();
}

fn delete(db: &db::Conn) {
    let cur_user = users::get_current_username()
        .unwrap()
        .into_string()
        .unwrap();

    println!();
    println!("ID of the post to delete?");
    let mut id_num_in = String::new();
    io::stdin().read_line(&mut id_num_in).unwrap();
    let id_num_in: u32 = id_num_in.trim().parse().unwrap();
    println!();

    let del_stmt = format!("DELETE FROM posts WHERE id = {}", id_num_in);
    let get_stmt = format!("SELECT * FROM posts WHERE id = {}", id_num_in);

    let mut get_stmt = db.conn.prepare(&get_stmt).unwrap();
    let mut del_stmt = db.conn.prepare(&del_stmt).unwrap();

    let user_in_post: String = get_stmt
        .query_row(rusqlite::NO_PARAMS, |row| row.get(2))
        .unwrap();

    if cur_user != user_in_post {
        println!("Users don't match. Can't delete!");
        println!();
        return;
    }

    posts::exec_stmt_no_params(&mut del_stmt).unwrap();
}
