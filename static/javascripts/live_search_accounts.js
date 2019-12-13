function generateRow(json) {
    row = document.createElement("tr");

    cell = document.createElement("td");
    row.appendChild(cell);
    cell.innerHTML = json.name_search;

    cell = document.createElement("td");
    row.appendChild(cell);
    cell.innerHTML = json.mail_search;

    cell = document.createElement("td");
    cell.style = "text-transform: lowercase;";
    row.appendChild(cell);
    if (json.permission != "DEFAULT") {
        cell.innerHTML = json.permission_search;
    }

    cell = document.createElement("td");
    row.appendChild(cell);
    link = document.createElement("a");
    cell.appendChild(link);
    link.textContent = "Edit";
    link.href = "/account/" + json.id;
    link = document.createElement("a");
    cell.appendChild(link);
    link.textContent = "Transactions";
    link.href = "/transactions/" + json.id;

    return row;
}

function updateTable(json) {
    tbody = document.getElementById("search-results");

    while(tbody.firstChild){
        tbody.removeChild(tbody.firstChild);
    }

    for (line of json) {
        row = generateRow(line);
        tbody.appendChild(row);
    }
}

function query(search) {
    component = encodeURIComponent(search).replace("%20", "+");
    window.history.replaceState(null, "", "/accounts?search=" + component);
    fetch("/api/v1/accounts?search=" + component)
    .then((response) => {
        if (!response.ok) {
            throw new Error('HTTP error, status = ' + response.status);
        }

        return response.json();
    })
    .then((json) => {
        updateTable(json);
    });
}

window.addEventListener('DOMContentLoaded', (event) => {
    input = document.getElementById("search-input");

    input.addEventListener("change", (event) => { 
        query(input.value) 
    });

    input.addEventListener("keyup", (event) => { 
        query(input.value) 
    });
});
