


pub trait Printer {
    fn output_line(&self, line: String);
    fn new_line(&self);

    //Prints  a formatted table given a vector of its rows (note that any headers must simply be passed as rows/columns)
    fn output_table(&self, table: &Vec<Vec<String>>);
}


pub struct CLIPrinter {}
impl Printer for CLIPrinter {
    fn output_line(&self, line: String) {
        println!("{}", line);
    }
    fn new_line(&self) {
        println!();
    }

    fn output_table(&self, table: &Vec<Vec<String>>) {
        for row in table {
            for entry in row {
                print!("{:<17}", entry);
            }
            println!();
        }
    }
}