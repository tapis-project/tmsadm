#![forbid(unsafe_code)]

use lazy_static::lazy_static;
use structopt::StructOpt;
use strum_macros::EnumString;
use std::path::Path;
use std::ops::Deref;
use std::io;

use path_absolutize::Absolutize;
use std::process::{Command, ExitStatus};
use execute::Execute;

// ***************************************************************************
//                             Constants
// ***************************************************************************
const TMSADM_INFO: &str = concat!("
The tmsadm program provides administrative access to the TMS Server's Sqlite 
database from the command line. Access to this program should be limited to 
those that can logon to the TMS Server machine.  Administrators can list or 
delete records from several database tables. 

The sqlite3 program must be on the PATH for execution to succeed.
----------------------------------------------------------------------------");

const DEBUG: bool = true;

// Sqlite command line program that we call to access the database.
// Usage: sqlite3 [OPTIONS] FILENAME [SQL]
//   FILENAME is the name of an SQLite database. A new database is created
//   if the file does not previously exist, which we short-circuit.
const SQLITE3: &str = "sqlite3";

// SQL command prototypes.
const LIST_PUBKEY:       &str = "SELECT * FROM pubkeys ";
const LIST_CLIENT:       &str = "SELECT * FROM clients ";
const LIST_DELEGATION:   &str = "SELECT * FROM delegations ";
const DELETE_PUBKEY:     &str = "DELETE FROM pubkeys ";
const DELETE_CLIENT:     &str = "DELETE FROM clients ";
const DELETE_DELEGATION: &str = "DELETE FROM delegations ";

// ***************************************************************************
//                             Static Variables
// ***************************************************************************
// Assign the command line arguments BEFORE RUNTIME_CTX is initialized in main.
lazy_static! {
    pub static ref TMSADM_ARGS: TmsadmArgs = init_tmsadm_args();
}

// ***************************************************************************
//                                 Enums
// ***************************************************************************
#[derive(Debug, PartialEq, EnumString)]
pub enum TmsOperation {
    #[strum(ascii_case_insensitive)]
    LIST,
    #[strum(ascii_case_insensitive)]
    DELETE,
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, EnumString)]
pub enum TmsResource {
    #[strum(ascii_case_insensitive)]
    pubkey,
    #[strum(ascii_case_insensitive)]
    client,
    #[strum(ascii_case_insensitive)]
    delegation,
}

// ***************************************************************************
//                               Main Processing
// ***************************************************************************
fn main() {
    // Parse command line args and print based on compile-time debug flag.
    let arg_info = format!("*** Command line arguments *** \n{:?}\n", *TMSADM_ARGS);
    if DEBUG {println!("{}", arg_info);}

    // Check that the database file exists, which avoids sqlite3 creating it.
    check_db_file();

    // Choose the command processor to execute.
    if TMSADM_ARGS.operation == TmsOperation::LIST {
        // LIST operations.
        if TMSADM_ARGS.resource == TmsResource::pubkey {
            process_list_pubkey();
        } else if TMSADM_ARGS.resource == TmsResource::client {
            process_list_client();
        } else {
            process_list_delegation();
        }
    } else {
        // DELETE operations.
        if TMSADM_ARGS.resource == TmsResource::pubkey {
            process_delete_pubkey();
        } else if TMSADM_ARGS.resource == TmsResource::client {
            process_delete_client();
        } else {
            process_delete_delegation();
        }
    }
}

// ---------------------------------------------------------------------------
// process_list_pubkey:
// ---------------------------------------------------------------------------
fn process_list_pubkey() {
    // Construct the command and run it.
    let cmd = make_sqlite3_cmd(LIST_PUBKEY);
    run_command(cmd, "LIST pubkeys");
}

// ---------------------------------------------------------------------------
// process_list_client:
// ---------------------------------------------------------------------------
fn process_list_client() {
    // Construct the command and run it.
    let cmd = make_sqlite3_cmd(LIST_CLIENT);
    run_command(cmd, "LIST clients");
}

// ---------------------------------------------------------------------------
// process_list_delegation:
// ---------------------------------------------------------------------------
fn process_list_delegation() {
    // Construct the command and run it.
    let cmd = make_sqlite3_cmd(LIST_DELEGATION);
    run_command(cmd, "LIST delegations");
}

// ---------------------------------------------------------------------------
// process_delete_pubkey:
// ---------------------------------------------------------------------------
fn process_delete_pubkey() {
    if !TMSADM_ARGS.confirm_delete_off {
        // First show what could be deleted.
        process_list_pubkey();
        if !confirm_delete() {return}
    }

    // Construct the command and run it.
    let cmd = make_sqlite3_cmd(DELETE_PUBKEY);
    run_command(cmd, "DELETE pubkeys");
}

// ---------------------------------------------------------------------------
// process_delete_client:
// ---------------------------------------------------------------------------
fn process_delete_client() {
    if !TMSADM_ARGS.confirm_delete_off {
        // First show what could be deleted.
        process_list_client();
        if !confirm_delete() {return}
    }

    // Construct the command and run it.
    let cmd = make_sqlite3_cmd(DELETE_CLIENT);
    run_command(cmd, "DELETE clients");

}

// ---------------------------------------------------------------------------
// process_delete_delegation:
// ---------------------------------------------------------------------------
fn process_delete_delegation() {
    if !TMSADM_ARGS.confirm_delete_off {
        // First show what could be deleted.
        process_list_delegation();
        if !confirm_delete() {return}
    }

    // Construct the command and run it.
    let cmd = make_sqlite3_cmd(DELETE_DELEGATION);
    run_command(cmd, "DELETE delegations");
}

// ---------------------------------------------------------------------------
// make_sqlite3_cmd:
// ---------------------------------------------------------------------------
/** Create the command object that issues an OS call with this format:
 * 
 *   sqlite3 [OPTIONS] FILENAME [SQL]
 */
fn make_sqlite3_cmd(sql_stmt: &str) -> Command {
    // Construct the SQL command.
    let mut sql = sql_stmt.to_string();
    match &TMSADM_ARGS.sqlwhere {
        Some(wh) => sql += wh,
        None => {},
    }
    if TMSADM_ARGS.limit > 0 {
        sql += " LIMIT ";
        sql += TMSADM_ARGS.limit.to_string().as_str();
    }

    // Build the command with user selected options.
    let mut cmd = Command::new(SQLITE3);
    if !&TMSADM_ARGS.json_off {cmd.arg("-json");}
    if !&TMSADM_ARGS.header_off {cmd.arg("-header");}
    if !&TMSADM_ARGS.echo_off {cmd.arg("-echo");}
    cmd.arg(get_absolute_path(&TMSADM_ARGS.dbpath));
    cmd.arg(sql);
    cmd
}

// ***************************************************************************
//                               Config Structs
// ***************************************************************************
// ---------------------------------------------------------------------------
// init_tms_args:
// ---------------------------------------------------------------------------
/** Get the command line arguments. */
fn init_tmsadm_args() -> TmsadmArgs {
    TmsadmArgs::from_args()
}

// ---------------------------------------------------------------------------
// TmsadmArgs:
// ---------------------------------------------------------------------------
#[derive(Debug, StructOpt)]
#[structopt(name = "tmsadm", about = "Command line arguments for tmsadm program.", before_help = TMSADM_INFO)]
pub struct TmsadmArgs {
    /// Specify the operation to carry out.
    /// 
    #[structopt(short, long, possible_values=&["LIST","DELETE"])]
    pub operation: TmsOperation,

    /// Specify the resource type to which the operation will be applied.
    /// 
    #[structopt(short, long, possible_values=&["pubkey","client","delegation"])]
    pub resource: TmsResource,

    /// Path to TMS database file.
    /// 
    #[structopt(short, long, default_value="~/.tms/database/tms.db")]
    pub dbpath: String,

    /// Set JSON formatting (default=false, implying json is on).
    /// 
    #[structopt(short, long)]
    pub json_off: bool,

    /// Echo the SQL command in the output (default=false, implying echo on).
    /// 
    #[structopt(short, long)]
    pub echo_off: bool,

    /// Retrieve SQL column headings when using non-JSON format (default=false, implying headers on).
    /// 
    #[structopt(short, long)]
    pub header_off: bool,

    /// Limit the number of records returned. The default is 0 (no limit).
    /// 
    #[structopt(short, long, default_value = "0")]
    pub limit: i32,

    /// Don't prompt user for confirmation on deletes (default=false, implying conformation on).
    /// 
    #[structopt(long)]
    pub confirm_delete_off: bool,

    /// Provide an SQL WHERE clause to be submitted as part of a SQL statement. The clause
    /// must start with the word "WHERE" (case insensitive) be written exactly as it would 
    /// appear in an SQL statment. Example:
    /// 
    ///   "WHERE tms_user_id = 'bud' and host = 'example.com'"
    /// 
    /// Use the LIST operation to discover the columns that can be referenced for a chosen
    /// resource. Discovery can use JSON or non-JSON formatting and "--limit 1" to minimize
    /// output.
    /// 
    #[structopt(short, long)]
    pub sqlwhere: Option<String>,
}

// ***************************************************************************
//                               Utilities
// ***************************************************************************
// ---------------------------------------------------------------------------
// check_db_file:
// ---------------------------------------------------------------------------
fn check_db_file() {
    if !Path::new(&get_absolute_path(&TMSADM_ARGS.dbpath)).is_file() {
        panic!("Database file does not exist: {}",get_absolute_path(&TMSADM_ARGS.dbpath));
    }
}

// ---------------------------------------------------------------------------
// confirm_delete:
// ---------------------------------------------------------------------------
/** Prompt user to confirm deletion and read user input from stdin.
 */
fn confirm_delete() -> bool {
    // Prompt user for confirmation.
    println!("Confirm deletion of the above listed records (y/n):");

    // Collect response.
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            if input.to_lowercase().starts_with('y') {true}
                else {false}
        },
        Err(e) => panic!("error: {}", e),
    }
}

// ---------------------------------------------------------------------------
// get_absolute_path:
// ---------------------------------------------------------------------------
/** Replace tilde (~) and environment variable values in a path name and
 * then construct the absolute path name.  The difference between 
 * absolutize and standard canonicalize methods is that absolutize does not 
 * care about whether the file exists and what the file really is.
 * 
 * Here's a short version of how canonicalize would be used: 
 * 
 *   let p = shellexpand::full(path).unwrap();
 *   fs::canonicalize(p.deref()).unwrap().into_os_string().into_string().unwrap()
 * 
 * We have the option of using these to two ways to generate a String from the
 * input path (&str):
 * 
 *   path.to_owned()
 *   path.deref().to_string()
 * 
 * I went with the former on a hunch that it's the most appropriate, happy
 * to change if my guess is wrong.
 */
fn get_absolute_path(path: &str) -> String {
    // Replace ~ and environment variable values if possible.
    // On error, return the string version of the original path.
    let s = match shellexpand::full(path) {
        Ok(x) => x,
        Err(_) => return path.to_owned(),
    };

    // Convert to absolute path if necessary.
    // Return original input on error.
    let p = Path::new(s.deref());
    let p1 = match p.absolutize() {
        Ok(x) => x,
        Err(_) => return path.to_owned(),
    };
    let p2 = match p1.to_str() {
        Some(x) => x,
        None => return path.to_owned(),
    };

    p2.to_owned()
}

// ---------------------------------------------------------------------------
// run_command:
// ---------------------------------------------------------------------------
/** Make an operating system call and return an Output object that contains
 * the result code and stdout/stderr as vectors.  If the command cannot be run
 * or if it runs and returns a non-zero exit code, this method writes the log 
 * before returning an error.  
 * 
 * The task parameter prefixes any error message logged or returned by this
 * function.
 * 
 * The only way Ok is returned is when the command has a zero exit code.
 */
#[allow(clippy::needless_return)]
fn run_command(mut command: Command, task: &str) {
    // Capture all output.
    //command.stdout(Stdio::piped());
    //command.stderr(Stdio::piped());
 
    // Return an output object or error.
    // Errors are logged before returning.
    match command.execute_output() {
        Ok(o) => {
            // Check for success here.
            if o.status.success() {}
                else {
                    let msg = task.to_string() + ": " + 
                        &String::from_utf8(o.stderr)
                        .unwrap_or(run_command_emsg(command, o.status));
                    panic!("{}", msg);
                };
        },
        Err(e) => {
            let msg = task.to_string() + ": " + &e.to_string();
            panic!("{}", msg);
        },
    };
}

// ---------------------------------------------------------------------------
// run_command_emsg:
// ---------------------------------------------------------------------------
/** Return a message for commands that return non-zero exit codes. */
fn run_command_emsg(command: Command, status: ExitStatus) -> String {
    "Unknown error condition returned by command: ".to_owned() + 
    command.get_program().to_str().unwrap_or("unknown") +
    " with exit status: " + &status.to_string()
}