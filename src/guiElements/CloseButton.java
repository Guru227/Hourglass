package guiElements;

import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;

import constants.Constants;

//Closes window without terminating the program (Timer still runs)
public class CloseButton extends MenuButton{
	/**
	 * 
	 */
	private static final long serialVersionUID = -7986051705687255007L;

	public CloseButton(String m){
		super();
		setText(m);		//Set message on button
		addActionListener(new ActionListener() {
			public void actionPerformed(ActionEvent e) {
				Constants.b1.setEnabled(false);
				Constants.parentJDialog.dispose();
			}
		});
	}
}

/*
private class CloseOnClick implements ActionListener{	
	//uses input parent window's handler to close window
	public void actionPerformed(ActionEvent e, JDialog parentJDialog)
  {
		Constants.continueTimer = true;
		System.out.println("Set continueTimer variable in Constants");
		System.out.println(Constants.continueTimer);
		parentJDialog.dispose();
		//windowClosing()
  }
}
*/