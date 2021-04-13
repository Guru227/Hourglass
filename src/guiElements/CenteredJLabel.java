package guiElements;

import java.awt.Font;
import java.awt.GridBagLayout;

import javax.swing.JLabel;
import javax.swing.JPanel;

import constants.CurrentConfiguration;

public class CenteredJLabel extends JPanel{
    /**
	 * 
	 */
	private static final long serialVersionUID = 830177309552899690L;
	private GridBagLayout layout = new GridBagLayout();
    private JLabel label;

    public CenteredJLabel(String message, Font labelFont) {
    	label = new JLabel(message);
    	label.setFont(labelFont);
		label.setForeground(CurrentConfiguration.fg);
		label.setBackground(CurrentConfiguration.labelBg);
        setLayout(layout);
        add(label);  	
    }
}