from . import schema
import typer


app = typer.Typer()
app.add_typer(schema.schema, name="schema", help="Generate a JSON schema for a type.")

if __name__ == "__main__":
    app()
