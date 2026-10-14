// <person-filter> wraps the "Reported by" <select>. With JavaScript the select gives way to a
// button that opens a sheet with a search box and everyone as radio buttons (name and role);
// picking someone sets the select. Without JavaScript the select works on its own.
class PersonFilter extends HTMLElement {
  connectedCallback() {
    const select = this.querySelector("select");
    if (!select || this.dataset.ready) {
      return;
    }
    this.dataset.ready = "";
    const button = document.createElement("button");
    button.type = "button";
    button.className = "button";
    button.dataset.variant = "secondary";
    button.textContent = select.selectedOptions[0]?.text ?? "";
    const dialog = this.sheet(select, button);
    button.addEventListener("click", () => dialog.showModal());
    select.hidden = true;
    this.append(button);
    document.body.append(dialog);
  }

  sheet(select, button) {
    const dialog = document.createElement("dialog");
    dialog.className = "sheet";
    dialog.setAttribute("aria-label", this.closest("label")?.firstElementChild?.textContent ?? "");
    const close = document.createElement("form");
    close.method = "dialog";
    const closer = document.createElement("button");
    closer.className = "button";
    closer.dataset.variant = "quiet";
    closer.textContent = this.dataset.close ?? "";
    close.append(closer);
    const search = document.createElement("input");
    search.type = "search";
    search.placeholder = this.dataset.search ?? "";
    search.setAttribute("aria-label", search.placeholder);
    const list = document.createElement("div");
    list.className = "rows";
    for (const option of select.options) {
      const label = document.createElement("label");
      label.className = "row";
      const radio = document.createElement("input");
      radio.type = "radio";
      radio.name = "person-filter";
      radio.value = option.value;
      radio.checked = option.selected;
      const name = document.createElement("span");
      name.className = "row-text";
      name.textContent = option.text;
      const role = document.createElement("span");
      role.className = "muted";
      role.textContent = option.dataset.role ?? "";
      label.append(radio, name, role);
      list.append(label);
    }
    search.addEventListener("input", () => {
      const query = search.value.trim().toLowerCase();
      for (const row of list.children) {
        row.hidden = !row.textContent.toLowerCase().includes(query);
      }
    });
    list.addEventListener("change", (event) => {
      select.value = event.target.value;
      button.textContent = select.selectedOptions[0]?.text ?? "";
      dialog.close();
    });
    dialog.append(close, search, list);
    return dialog;
  }
}

customElements.define("person-filter", PersonFilter);
